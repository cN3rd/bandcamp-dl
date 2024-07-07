#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(clippy::cargo)]
// #![deny(missing_docs)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::items_after_statements)]

use clap::Parser;

mod api;
mod cache;
mod cli;
mod cookies;
mod error;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cli::run_program(cli::Cli::try_parse()?).await
}
