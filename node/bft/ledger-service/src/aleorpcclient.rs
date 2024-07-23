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

use anyhow::{Context, Result};
use std::time::Duration;

use snarkvm::prelude::{Block, Deserialize, Network};

#[derive(Clone)]
pub struct AleoRpcClient {
    base_url: String,
    inner: reqwest::Client,
}

impl AleoRpcClient {
    pub fn new(base_url: &str) -> Self {
        // ex: https://vm.aleo.org/api/testnet3
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            inner: reqwest::Client::builder().timeout(Duration::from_secs(5)).build().unwrap(),
        }
    }

    pub async fn get_resource<R: for<'a> Deserialize<'a>>(&self, url: &str) -> Result<R> {
        let resp = self.inner.get(url).send().await?;
        let status = resp.status();
        let data = resp.text().await.context("get resource to text")?;
        let resource = match status.is_success() {
            true => serde_json::from_str::<R>(&data).with_context(move || format!("serialize data to resource: {}", data))?,
            false => return Err(anyhow::anyhow!("request {} failed, status: {}, body: {}", &url, status, data)),
        };
        Ok(resource)
    }
}

impl AleoRpcClient {
    pub async fn get_block<N: Network>(&self, block_height: u32) -> Result<Block<N>> {
        let url = format!("{}/block/{}", self.base_url, block_height);
        let block = self.get_resource(&url).await?;
        Ok(block)
    }
}



