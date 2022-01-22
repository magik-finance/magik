use anchor_client::{solana_client::rpc_client::RpcClient, Client, ClientError};
use anchor_lang::prelude::*;
use clap::{Result, SubCommand};
use port_variable_rate_lending_instructions;
use port_variable_rate_lending_instructions::{instruction::LendingInstruction, state::Obligation};
use solana_sdk::program_pack::Pack;
use solana_sdk::{
    commitment_config,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
    system_instruction::create_account,
    system_program, sysvar,
    transaction::Transaction,
};
use std::sync::{Arc, Mutex};
use std::{mem::size_of, rc::Rc, str::FromStr, thread, time::Duration};

use magik_program::{self, state};
fn main() {
    let matches = clap::App::new("Magik CLI toolkit")
        .version("1.0")
        .author("batphonghan")
        .about("Magik CLI toolkit")
        .subcommand(SubCommand::with_name("init_obligation"))
        .subcommand(SubCommand::with_name("crank"))
        .arg(
            clap::Arg::with_name("lending_market")
                .long("lending_market")
                .default_value("H27Quk3DSbu55T4dCr1NddTTSAezXwHU67FPCZVKLhSW"),
        )
        .arg(
            clap::Arg::with_name("lending_program_id")
                .long("lending_program_id")
                .default_value("pdQ2rQQU5zH2rDgZ7xH2azMBJegUzUyunJ5Jd637hC4"),
        )
        .arg(
            clap::Arg::with_name("mint_token")
                .long("mint_token")
                .default_value("So11111111111111111111111111111111111111112"), //Reserve Public Keys USDC
        )
        .arg(
            clap::Arg::with_name("program_id")
                .long("program_id")
                .default_value("CmHZHMPRfsNpYZe1YUJA59EaCfZiQqJAyBrx9oA3QtCg"),
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

    let cluster_url = matches.value_of("cluster").unwrap().to_string();
    println!("Value for cluster: {}", &cluster_url);

    let program_id_str = matches.value_of("program_id").unwrap();
    println!("Value for program ID: {}", program_id_str);

    let magik_program = Pubkey::from_str(program_id_str).unwrap();

    let payer = read_keypair_file(wallet.clone()).expect("Requires a keypair file");

    let cluster = anchor_client::Cluster::from_str(cluster_url.as_str()).unwrap();

    let client = Client::new_with_options(
        cluster,
        Rc::new(payer),
        commitment_config::CommitmentConfig::processed(),
    );

    let client = Mutex::new(Arc::new(client));
    let lending_program_id = matches.value_of("lending_program_id").unwrap();
    let lending_program = Pubkey::from_str(lending_program_id).unwrap();

    let lending_market_str = matches.value_of("lending_market").unwrap();
    let lending_market = Pubkey::from_str(lending_market_str).unwrap();

    let mint_token = Pubkey::from_str(matches.value_of("mint_token").unwrap()).unwrap();

    let authority = read_keypair_file(wallet.clone()).expect("Requires a keypair file");

    let (vault, vault_bump) = Pubkey::find_program_address(
        &[b"vault", mint_token.as_ref(), authority.pubkey().as_ref()],
        &magik_program,
    );

    let (vault_token, token_bump) = Pubkey::find_program_address(
        &[b"vault_token", mint_token.as_ref(), vault.as_ref()],
        &magik_program,
    );
    let (synth_mint, mint_bump) = Pubkey::find_program_address(
        &[b"synth_mint", mint_token.as_ref(), vault.as_ref()],
        &magik_program,
    );
    println!("Value for magik_program: {}", &magik_program);
    let space = Obligation::LEN;
    let nonce = Keypair::new().pubkey();

    let (obligation, ob_bump) = Pubkey::find_program_address(
        &[b"obligation", nonce.as_ref(), vault.as_ref()],
        &magik_program,
    );

    let port_program = lending_program;
    let source_liquidity = obligation;
    let destination_collateral = obligation;
    let reserve = obligation;
    let reserve_liquidity_supply = obligation;
    let reserve_collateral_mint = obligation;
    let lending_market_authority = obligation;
    let transfer_authority = vault;
    match matches.subcommand_name() {
        Some("crank") => {
            let lending_handler = thread::spawn(move || {
                let authority = read_keypair_file(wallet.clone()).expect("Requires a keypair file");
                let wallet = authority.to_bytes();
                let cluster = anchor_client::Cluster::from_str(cluster_url.as_str()).unwrap();
                let client = Client::new_with_options(
                    cluster,
                    Rc::new(authority),
                    commitment_config::CommitmentConfig::processed(),
                );
                loop {
                    thread::sleep(Duration::from_secs(5));
                    let authority = Keypair::from_bytes(&wallet).unwrap();
                    let magik_client = client.program(magik_program);
                    let rs = magik_client
                        .request()
                        .accounts(magik_program::accounts::LendingCrank {
                            vault,
                            port_program,
                            source_liquidity,
                            destination_collateral,
                            reserve,
                            reserve_liquidity_supply,
                            reserve_collateral_mint,
                            lending_market,
                            lending_market_authority,
                            transfer_authority,
                            token_program: spl_token::ID,
                            clock: sysvar::clock::ID,
                        })
                        .args(magik_program::instruction::LendingCrank {})
                        .signer(&authority)
                        .send();
                    println!("TX crank lending: {:?} obligation {}", rs, obligation);
                    assert_eq!(rs.is_err(), false);
                }
            });
            let harvest_handler = thread::spawn(move || {
                // TODO: Calling harvest onchain
            });

            lending_handler.join().unwrap();
            harvest_handler.join().unwrap();
        }
        Some("init_obligation") => {
            let magik_client = client.lock().unwrap().program(magik_program);
            let lamports = magik_client
                .rpc()
                .get_minimum_balance_for_rent_exemption(space)
                .unwrap();

            println!(
                "Lamport {} space {} ob {} nonce {}",
                lamports, space, obligation, nonce
            );
            let rs = magik_client
                .request()
                .accounts(magik_program::accounts::Init {
                    vault,
                    vault_token,
                    synth_mint,
                    mint_token,
                    authority: authority.pubkey(),
                    obligation,
                    lending_market,
                    system_program: system_program::id(),
                    lending_program,
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
                        percent: 40,
                    },
                    ob_bump,
                    nonce,
                })
                .signer(&authority)
                .send();
            println!("TX magik_client INIT: {:?} obligation {}", rs, obligation);
            assert_eq!(rs.is_err(), false);
        }
        _ => println!("Unsupported command"),
    }
}
