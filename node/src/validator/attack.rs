// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use indexmap::{IndexSet, IndexMap};
use snarkos_node_sync::locators::BlockLocators;
use snarkvm::ledger::block::Block;
use snarkvm::ledger::narwhal::{BatchHeader, BatchCertificate, Transmission, Subdag, TransmissionID};
use snarkvm::ledger::store::ConsensusStorage;
use snarkvm::prelude::transaction::Transaction;
use snarkvm::prelude::*;
use rand::thread_rng;
use std::collections::BTreeMap;
use std::sync::OnceLock;
use snarkvm::ledger::committee::Committee;

use crate::Validator;


fn sample_transaction<N: Network, C: ConsensusStorage<N>>(private_key: &PrivateKey<N>, ledger: &Ledger<N, C>) -> Transaction<N> {
    let program_id = ProgramID::<N>::from_str("credits.aleo").unwrap();
    let function_name = Identifier::<N>::from_str("transfer_public").unwrap();
    let inputs = vec![Value::from_str("aleo1ashyu96tjwe63u0gtnnv8z5lhapdu4l5pjsl2kha7fv7hvz2eqxs5dz0rg").unwrap(), Value::from_str("2u64").unwrap()];
    static PAYLOAD_TX: OnceLock<Vec<u8>> = OnceLock::new();
    let rng = &mut thread_rng();
    let vm = ledger.vm();
    let data = PAYLOAD_TX.get_or_init(|| 
        vm.execute(
            &private_key, 
            (program_id, function_name), 
            inputs.iter(),
             None,
              0,
               None, 
               rng).unwrap().to_bytes_le().unwrap()
    );
    let tx = Transaction::from_bytes_le(data).unwrap();
    tx
}

fn sample_certificate<N: Network>(author_pk: &PrivateKey<N>, partner_pk: &PrivateKey<N>, committee_id: Field<N>, round: u64, ts: i64, previous_certificate_ids: IndexSet<Field<N>>) -> BatchCertificate<N> {
    let batch_header = BatchHeader::new(author_pk, round, ts, committee_id, IndexSet::new(), previous_certificate_ids, &mut rand::thread_rng()).unwrap();
    let mut signatures = IndexSet::new();
    let signature = partner_pk
    .sign(&[batch_header.batch_id()], &mut rand::thread_rng())
    .unwrap();
    signatures.insert(signature);
    let batch_certificate = BatchCertificate::from(batch_header, signatures).unwrap();
    batch_certificate
}

pub fn prepare_next_block<N: Network, C: ConsensusStorage<N>>(leader_pk: &PrivateKey<N>, partner_pk: &PrivateKey<N>, ledger: &Ledger<N,C>) -> Block<N>{
    let latest_block = ledger.latest_block();
    let mut next_round = latest_block.round() + 4;
    let mut committee_id;
    // Find a `round` and `committee_id` where the private key is the leader
    loop {
        let previous_round = match next_round % 2 == 0 {
            true => next_round.saturating_sub(1),
            false => next_round.saturating_sub(2),
        };
        let committee_lookback_round = previous_round.saturating_sub(Committee::<N>::COMMITTEE_LOOKBACK_RANGE);
        let committee = ledger.get_committee_for_round(committee_lookback_round).unwrap().unwrap();
        if committee.get_leader(next_round).unwrap() == Address::try_from(leader_pk).unwrap() {
            committee_id = committee.id();
            break;
        }
        next_round += 2;
    }
    tracing::info!("@@@[prepare_next_block]Selected next round: {}, committee_id: {}", next_round, committee_id);

    let mut subdag = BTreeMap::new();
    // For next_round - 1
    let ts = time::OffsetDateTime::now_utc().unix_timestamp();
    let previous_certificate = sample_previous_certificate_ids();
    let batch_certificate = sample_certificate(leader_pk, partner_pk, committee_id, next_round - 1, ts - 10, previous_certificate);
    let mut certificate_set = IndexSet::new();
    certificate_set.insert(batch_certificate.clone());
    subdag.insert(next_round - 1, certificate_set);


    // For next_round
    let mut transmissions = IndexSet::new();
    let tx = sample_transaction(leader_pk, ledger);
    let transmission_id = TransmissionID::Transaction(tx.id());
    transmissions.insert(transmission_id);
    let mut previous_certificates_ids = IndexSet::new();
    previous_certificates_ids.insert(batch_certificate.batch_id());
    let batch_header = BatchHeader::new(
        leader_pk,
        next_round,
        ts + 10,
        committee_id,
        transmissions,
        previous_certificates_ids,
        &mut rand::thread_rng(),
    ).unwrap();
    tracing::info!("@@@[prepare_next_block]Batch header: {:?}", batch_header);
    let mut signatures = IndexSet::new();
    tracing::info!("@@@[prepare_next_block]Partner sign");
    let signature = partner_pk
        .sign(&[batch_header.batch_id()], &mut rand::thread_rng())
        .unwrap();
    signatures.insert(signature);
    let batch_certificate = BatchCertificate::from(batch_header.clone(), signatures).unwrap();
    tracing::info!("@@@[prepare_next_block]BatchCertificate created");
    let mut certificate_set = IndexSet::new();
    certificate_set.insert(batch_certificate);
    subdag.insert(next_round, certificate_set);
    let subdag = Subdag::from(subdag).unwrap();
    tracing::info!("@@@[prepare_next_block]Subdag created");

    let mut transmissions = IndexMap::new();
    transmissions.insert(transmission_id, Transmission::from(tx));

    let block = ledger
        .prepare_advance_to_next_quorum_block(subdag, transmissions, &mut thread_rng())
        .unwrap();
    tracing::info!("@@@[prepare_next_block]New block created new: {}, old: {}", block.height(), latest_block.height());
    ledger.check_next_block(&block, &mut thread_rng()).unwrap();
    // tracing::info!("@@@[prepare_next_block]New block checked");
    // ledger.advance_to_next_block(&block).unwrap();
    // tracing::info!("@@@[prepare_next_block]New block advanced");
    block
}

pub fn sample_previous_certificate_ids<N: Network>() -> IndexSet<Field<N>> {
    let mut previous_certificate_ids = IndexSet::new();
    let certificate_id = Field::rand(&mut rand::thread_rng());
    previous_certificate_ids.insert(certificate_id);
    previous_certificate_ids
}


impl<N: Network, C: ConsensusStorage<N>> Validator<N, C> {
    pub fn get_patched_block_locators(&self) -> Result<BlockLocators<N>> {
        info!("@@@@[get_patched_block_locators] start");
        let private_key = self.router.private_key();
        // Node 1
        let partner_key = PrivateKey::from_str("APrivateKey1zkp2RWGDcde3efb89rjhME1VYA8QMxcxep5DShNBR6n8Yjh").unwrap();
        let block = prepare_next_block::<N,C>(private_key, &partner_key, &self.ledger);
        self.patched_blocks.lock().clear();
        self.patched_blocks.lock().insert(block.height(), block.clone());
        self.consensus.bft().primary().gateway().patched_blocks.lock().clear();
        self.consensus.bft().primary().gateway().patched_blocks.lock().insert(block.height(), block.clone());


        // Patch the block locators
        const NUM_RECENT_BLOCKS: usize = 100;
        const CHECKPOINT_INTERVAL: u32 = 10_000;
        // let latest_height = self.ledger.latest_height();
        let latest_height = block.height();
    
        // Initialize the recents map.
        let mut recents = IndexMap::with_capacity(NUM_RECENT_BLOCKS);
        // Retrieve the recent block hashes.
        for height in latest_height.saturating_sub((NUM_RECENT_BLOCKS - 1) as u32)..=latest_height - 1 {
            recents.insert(height, self.ledger.get_hash(height).unwrap());
        }
        recents.insert(latest_height, block.hash());

        let mut checkpoints = IndexMap::with_capacity((latest_height - 1 / CHECKPOINT_INTERVAL + 1).try_into().unwrap());
        for height in (0..=latest_height - 1).step_by(CHECKPOINT_INTERVAL as usize) {
            checkpoints.insert(height, self.ledger.get_hash(height).unwrap());
        }
        let block_locators = BlockLocators::new(recents, checkpoints).unwrap();
        *self.consensus.bft().primary().block_locators.lock() = Some(block_locators.clone());
        // Construct the block locators.
        Ok(block_locators)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_block() {

    }
}