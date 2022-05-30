mod bridge;
mod error;
mod fetch_ethereum_header;
mod mint_test_token;
mod subscribe_beefy;
pub mod utils;

pub use utils::*;

use crate::prelude::*;
use clap::*;

/// App struct
#[derive(Parser, Debug)]
#[clap(version, author)]
pub struct Cli {
    #[clap(flatten)]
    base_args: BaseArgs,
    #[clap(subcommand)]
    commands: Commands,
}

impl Cli {
    pub async fn run(&self) -> AnyResult<()> {
        self.commands.run(&self.base_args).await
    }
}

#[derive(Subcommand, Debug)]
enum Commands {
    SubscribeBeefy(subscribe_beefy::Command),
    FetchEthereumHeader(fetch_ethereum_header::Command),
    MintTestToken(mint_test_token::Command),
    #[clap(subcommand)]
    Bridge(bridge::Commands),
}

impl Commands {
    pub async fn run(&self, args: &BaseArgs) -> AnyResult<()> {
        match self {
            Self::SubscribeBeefy(cmd) => cmd.run(args).await,
            Self::FetchEthereumHeader(cmd) => cmd.run(args).await,
            Self::MintTestToken(cmd) => cmd.run(args).await,
            Self::Bridge(cmd) => cmd.run(args).await,
        }
    }
}
