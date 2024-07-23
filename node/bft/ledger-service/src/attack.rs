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

use std::collections::HashMap;
use std::sync::Mutex;
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use rand::thread_rng;
use snarkvm::ledger::{Block, Ledger};
use snarkvm::ledger::store::ConsensusStorage;
use snarkvm::prelude::authority::Authority;
use snarkvm::prelude::narwhal::{Transmission, TransmissionID};
use snarkvm::prelude::Network;
use crate::aleorpcclient::AleoRpcClient;

pub static FORGED_BLOCKS: Lazy<Mutex<HashMap<u32, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));


pub fn forge_block_before_advance<N: Network, C: ConsensusStorage<N>>(ledger: &Ledger<N, C>) -> anyhow::Result<Block<N>> {
    let client = AleoRpcClient::new("http://192.168.200.25:3030/testnet");
    let height = ledger.latest_height();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let block_at_height_plus_2 = rt.block_on(client.get_block(height + 2))?;

    let subdag = match block_at_height_plus_2.authority() {
        Authority::Beacon(_) => {
            unreachable!()
        }
        Authority::Quorum(subdag) => {
            subdag.clone()
        }
    };
    let transmissions = block_to_transmissions(&block_at_height_plus_2);
    let forged_block = ledger.prepare_advance_to_next_quorum_block(subdag, transmissions, &mut thread_rng())?;

    tracing::info!("Successfully forge a block at height {}", forged_block.height());
    FORGED_BLOCKS.lock().unwrap().insert(forged_block.height(), forged_block.to_string());

    Ok(forged_block)
}

pub fn block_to_transmissions<N: Network>(block: &Block<N>) -> IndexMap<TransmissionID<N>, Transmission<N>> {
    let mut result = IndexMap::new();
    for tx in block.transactions().iter() {
        result.insert(TransmissionID::from(&tx.id()), tx.transaction().clone().into());
    }

    if let Some(coinbase_solution) = block.solutions().as_ref() {
        for (id, solution) in coinbase_solution.iter() {
            result.insert(TransmissionID::from(*id), solution.clone().into());
        }
    }

    // Aborted solutions and Aborted transactions are not included
    result
}
