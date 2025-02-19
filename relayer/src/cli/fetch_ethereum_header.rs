// This file is part of the SORA network and Polkaswap app.

// Copyright (c) 2020, 2021, Polka Biome Ltd. All rights reserved.
// SPDX-License-Identifier: BSD-4-Clause

// Redistribution and use in source and binary forms, with or without modification,
// are permitted provided that the following conditions are met:

// Redistributions of source code must retain the above copyright notice, this list
// of conditions and the following disclaimer.
// Redistributions in binary form must reproduce the above copyright notice, this
// list of conditions and the following disclaimer in the documentation and/or other
// materials provided with the distribution.
//
// All advertising materials mentioning features or use of this software must display
// the following acknowledgement: This product includes software developed by Polka Biome
// Ltd., SORA, and Polkaswap.
//
// Neither the name of the Polka Biome Ltd. nor the names of its contributors may be used
// to endorse or promote products derived from this software without specific prior written permission.

// THIS SOFTWARE IS PROVIDED BY Polka Biome Ltd. AS IS AND ANY EXPRESS OR IMPLIED WARRANTIES,
// INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL Polka Biome Ltd. BE LIABLE FOR ANY
// DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
// BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS;
// OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT,
// STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
// USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use crate::cli::prelude::*;
use crate::ethereum::make_header;
use ethers::prelude::*;

#[derive(Args, Clone, Debug)]
pub(super) struct Command {
    /// Blocks until the Ethereum header is considered final
    #[clap(long, short)]
    descendants_until_final: Option<usize>,
    /// Block number to fetch
    #[clap(long, short)]
    number: Option<usize>,
    #[clap(flatten)]
    eth: EthereumClient,
}

impl Command {
    pub(super) async fn run(&self) -> AnyResult<()> {
        let client = self.eth.get_unsigned_ethereum().await?;
        let number = match (self.descendants_until_final, self.number) {
            (Some(v), None) => {
                let latest_block = client
                    .get_block(BlockId::Number(BlockNumber::Latest))
                    .await?
                    .unwrap();
                let number = latest_block.number.unwrap() - U64::from(v);
                number
            }
            (None, Some(v)) => U64::from(v),
            _ => return Err(anyhow::anyhow!("Invalid arguments")),
        };
        let finalized_block = client
            .get_block(BlockId::Number(BlockNumber::Number(number)))
            .await?
            .unwrap();
        let expected_hash = finalized_block.hash.unwrap_or_default();
        let header = make_header(finalized_block);
        let hash = header.compute_hash();
        info!("Hash: {:?}", hash);
        info!("Expected: {:?}", expected_hash);
        let result = serde_json::to_string(&header)?;
        println!("{}", result);
        Ok(())
    }
}
