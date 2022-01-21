use anchor_client::{solana_client::rpc_client::RpcClient, Client, ClientError};
use anchor_lang::prelude::*;
use clap::{Result, SubCommand};
use solana_sdk::{
    commitment_config, instruction::Instruction, pubkey::Pubkey, signature::read_keypair_file,
    signer::Signer, system_program, sysvar,
};
use std::{rc::Rc, str::FromStr, thread, time::Duration};

use magik_program::{self, state};
fn main() {
    let matches = clap::App::new("Magik CLI toolkit")
        .version("1.0")
        .author("batphonghan")
        .about("Magik CLI toolkit")
        .subcommand(
            SubCommand::with_name("init_obligation")
                .arg(
                    clap::Arg::with_name("obligation")
                        .long("obligation")
                        .default_value(""),
                )
                .arg(
                    clap::Arg::with_name("lending_market")
                        .long("lending_market")
                        .default_value("H27Quk3DSbu55T4dCr1NddTTSAezXwHU67FPCZVKLhSW"),
                )
                .arg(
                    clap::Arg::with_name("mint_token")
                        .long("mint_token")
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
            let matches = matches.subcommand_matches("init_obligation").unwrap();
            let lending_market_str = matches.value_of("lending_market").unwrap();
            let lending_market = Pubkey::from_str(lending_market_str).unwrap();

            let mint_token = Pubkey::from_str(matches.value_of("mint_token").unwrap()).unwrap();

            let authority = read_keypair_file(wallet.clone()).expect("Requires a keypair file");

            let (vault, vault_bump) = Pubkey::find_program_address(
                &[b"vault", mint_token.as_ref(), authority.pubkey().as_ref()],
                &program_id,
            );

            let (vault_token, token_bump) = Pubkey::find_program_address(
                &[b"vault_token", mint_token.as_ref(), vault.as_ref()],
                &program_id,
            );
            let (synth_token, mint_bump) = Pubkey::find_program_address(
                &[b"synth_token", mint_token.as_ref(), vault.as_ref()],
                &program_id,
            );

            let rs = program_client
                .request()
                .accounts(magik_program::accounts::Init {
                    vault,
                    vault_token,
                    synth_token,
                    mint_token,
                    authority: authority.pubkey(),
                    obligation: vault,
                    lending_market,
                    system_program: system_program::id(),
                    port_program: port_variable_rate_lending_instructions::id(),
                    token_program: spl_token::ID,
                    clock: sysvar::clock::ID,
                    rent: sysvar::rent::ID,
                })
                .args(magik_program::instruction::Init {
                    param: magik_program::state::InitParam {
                        bump: magik_program::state::Bump {
                            mint_bump,
                            token_bump,
                            vault_bump,
                        },
                        init_obligation: true,
                        percent: 50,
                    },
                })
                .signer(&authority)
                .send();
            println!("RS: {:?}", rs)
        }
        _ => println!("Unsupported command"),
    }
}
