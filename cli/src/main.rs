use anchor_client::{solana_client::rpc_client::RpcClient, Client, ClientError};
use anchor_lang::prelude::*;
use clap::{Result, SubCommand};
use solana_sdk::{
    commitment_config, instruction::Instruction, pubkey::Pubkey, signature::read_keypair_file,
    signer::Signer, system_program, sysvar,
};
use std::{rc::Rc, str::FromStr, thread, time::Duration};

fn main() {
    let matches = clap::App::new("Magik CLI toolkit")
        .version("1.0")
        .author("batphonghan")
        .about("Magik CLI toolkit")
        .subcommand(
            SubCommand::with_name("init_obligation")
                .arg(
                    clap::Arg::with_name("owners")
                        .long("obligation_owner")
                        .default_value(""),
                )
                .arg(
                    clap::Arg::with_name("obligation")
                        .long("obligation")
                        .default_value(""),
                )
                .arg(
                    clap::Arg::with_name("lending_market")
                        .long("lending_market")
                        .default_value(""),
                ),
        )
        .arg(
            clap::Arg::with_name("program_id")
                .long("program_id")
                .default_value(""),
        )
        .arg(
            clap::Arg::with_name("cluster")
                .short("c")
                .long("cluster")
                .default_value("https://api.devnet.solana.com"),
        )
        .arg(
            clap::Arg::with_name("wallet")
                .short("w")
                .long("wallet")
                .default_value("~/.config/solana/id.json"),
        )
        .get_matches();

    let wallet = matches.value_of("wallet").unwrap();
    let wallet = shellexpand::tilde(wallet).to_string();
    println!("Value for wallet: {}", wallet);

    let cluster_url = matches.value_of("cluster").unwrap();
    println!("Value for cluster: {}", &cluster_url);

    let program_id_str = matches.value_of("program_id").unwrap();
    println!("Value for program ID: {}", program_id_str);

    let program_id = Pubkey::from_str(program_id_str).unwrap();

    let payer = read_keypair_file(wallet.clone()).expect("Requires a keypair file");

    let cluster = anchor_client::Cluster::from_str(cluster_url).unwrap();

    let client = Client::new_with_options(
        cluster,
        Rc::new(payer),
        commitment_config::CommitmentConfig::processed(),
    );
    let program_client = client.program(program_id);

    match matches.subcommand_name() {
        Some("init_obligation") => {
            let payer = read_keypair_file(wallet.clone()).expect("Requires a keypair file");
        }
        _ => println!("Unsupported command"),
    }
}
