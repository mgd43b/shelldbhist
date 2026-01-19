mod cli;
mod cleanup;
mod config;
mod db;
mod domain;
mod template;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    cli::run(cli)
}