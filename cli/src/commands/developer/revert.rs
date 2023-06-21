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

use clap::Parser;
use snarkvm::synthesizer::{helpers::rocksdb::ConsensusDB, ConsensusStore, VM};
use anyhow::Result;
use super::CurrentNetwork;

#[derive(Debug, Parser)]
pub struct Revert {
    /// The program identifier.
    num: u32,
}
impl Revert {
    /// Executes an Aleo program function with the provided inputs.
    #[allow(clippy::format_in_format_args)]
    pub fn parse(self) -> Result<()> {
        let store = ConsensusStore::<CurrentNetwork, ConsensusDB<CurrentNetwork>>::open(None)?;
        let vm = VM::from(store)?;
        vm.block_store().remove_last_n(self.num)?;
        Ok(())
    }
}
