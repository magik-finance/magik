use anchor_client::{solana_client::rpc_client::RpcClient, Client, ClientError};
use anchor_lang::prelude::*;
use clap::{Result, SubCommand};
use port_variable_rate_lending_instructions;
use port_variable_rate_lending_instructions::{
    instruction::LendingInstruction, state as port_state,
};
use solana_clap_utils::input_parsers::pubkey_of;
use solana_sdk::account::ReadableAccount;
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
use spl_token::instruction::initialize_account;
use spl_token::{
    instruction::approve,
    state::{Account as Token, AccountState, Mint},
};
use std::sync::{Arc, Mutex};
use std::{mem::size_of, rc::Rc, str::FromStr, thread, time::Duration};

use magik_program::{self, state};
fn main() -> std::result::Result<(), ClientError> {
    let matches = clap::App::new("Magik CLI toolkit")
        .version("1.0")
        .author("batphonghan")
        .about("Magik CLI toolkit")
        .subcommand(SubCommand::with_name("init_obligation"))
        .subcommand(SubCommand::with_name("dst_collateral"))
        .subcommand(
            SubCommand::with_name("crank").arg(
                clap::Arg::with_name("obligation")
                    .long("obligation")
                    .default_value("6WmjCB141XT82BBEdjBPnpssf25CKzHv1cHSchDQFY1n"),
            ),
        )
        .arg(
            clap::Arg::with_name("reserve")
                .long("reserve")
                .default_value("6FeVStQAGPWvfWijDHF7cTWRCi7He6vTT3ubfNhe9SPt"),
        )
        .arg(
            clap::Arg::with_name("lending_program_id")
                .long("lending_program_id")
                .default_value("pdQ2rQQU5zH2rDgZ7xH2azMBJegUzUyunJ5Jd637hC4"),
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

    let cluster = anchor_client::Cluster::from_str(cluster_url.clone().as_str()).unwrap();

    let client = Client::new_with_options(
        cluster,
        Rc::new(payer),
        commitment_config::CommitmentConfig::processed(),
    );

    let client = Mutex::new(Arc::new(client));
    let lending_program = pubkey_of(&matches, "lending_program_id").unwrap();

    let rpc = RpcClient::new(cluster_url.clone());
    let reserve = pubkey_of(&matches, "reserve").unwrap();
    let reserve_data = rpc.get_account_data(&reserve).unwrap();
    let reserve_state = port_state::Reserve::unpack(&reserve_data).unwrap();
    println!("\n reserse DATA {:?} \n", reserve_state);

    let authority = read_keypair_file(wallet.clone()).expect("Requires a keypair file");

    let mint_token = reserve_state.liquidity.mint_pubkey;
    let lending_market = reserve_state.lending_market;
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

    println!("Magik_program: {}", &magik_program);
    println!("VAULT: {}", &vault);
    println!("Mint_token: {}", &mint_token);
    println!("Lending_market: {}", &lending_market);
    let space = port_state::Obligation::LEN;

    let reserve_collateral_mint = reserve_state.collateral.mint_pubkey;
    match matches.subcommand_name() {
        Some("crank") => {
            let matches = matches.subcommand_matches("crank").unwrap();
            let obligation = pubkey_of(&matches, "obligation").unwrap();
            let source_liquidity = vault_token;

            let reserve_liquidity_supply = reserve_state.liquidity.supply_pubkey;

            let port_program = lending_program;
            let transfer_authority = vault;
            let (lending_market_authority, _bump_seed) =
                Pubkey::find_program_address(&[&lending_market.as_ref()], &lending_program);

            /// Number of bytes in a pubkey
            pub const PUBKEY_BYTES: usize = 32;
            let (lending_market_authority_pubkey, _bump_seed) = Pubkey::find_program_address(
                &[&lending_market.to_bytes()[..PUBKEY_BYTES]],
                &lending_program,
            );

            let lending_handler = thread::spawn(move || {
                let _waller = wallet.clone();
                let authority = read_keypair_file(wallet.clone()).expect("Requires a keypair file");
                let authority_pubkey = authority.pubkey();
                let wallet = authority.to_bytes();
                let cluster = anchor_client::Cluster::from_str(cluster_url.as_str()).unwrap();
                let client = Client::new_with_options(
                    cluster,
                    Rc::new(authority),
                    commitment_config::CommitmentConfig::processed(),
                );
                let destination_collateral =
                    spl_associated_token_account::get_associated_token_address(
                        &vault,
                        &reserve_collateral_mint,
                    );
                if rpc.get_account_data(&destination_collateral).is_err() {
                    let authority = read_keypair_file(&_waller).expect("Requires a keypair file");
                    let hash = rpc.get_latest_blockhash().unwrap();
                    let create_token_acc_tx = Transaction::new_signed_with_payer(
                        &[
                            spl_associated_token_account::create_associated_token_account(
                                &authority_pubkey,
                                &vault,
                                &reserve_collateral_mint,
                            ),
                        ],
                        Some(&authority_pubkey),
                        &[&authority],
                        hash,
                    );
                    let sigs = rpc
                        .send_and_confirm_transaction(&create_token_acc_tx)
                        .unwrap();
                    println!(
                        "\n Newly create destination_collateral SIG: {} => {}",
                        sigs, destination_collateral
                    );
                } else {
                    println!(
                        "\n>>> Already have destination_collateral  {}",
                        destination_collateral
                    );
                }
                let dst_data = rpc.get_account_data(&destination_collateral).unwrap();
                let tk = Token::unpack(&dst_data).unwrap();
                println!(">>>>> TK {:?}", tk);

                println!(" vault {} ", vault);
                println!(" port_program {} ", port_program);
                println!(" reserve {} ", reserve);
                println!(" reserve_liquidity_supply {} ", reserve_liquidity_supply);
                println!(" reserve_collateral_mint {} ", reserve_collateral_mint);
                println!(" source_liquidity {} ", source_liquidity);
                println!(" lending_market {} ", lending_market);
                println!(" transfer_authority {} ", transfer_authority);
                println!(" destination_collateral {} ", destination_collateral);
                println!(
                    " lending_market_authority {} ",
                    lending_market_authority_pubkey
                );
                println!(" token_program {} ", spl_token::ID);
                println!(" clock {} ", sysvar::clock::ID);
                let mut c = 0;
                loop {
                    if c >= 1 {
                        break;
                    }
                    c += 1;
                    thread::sleep(Duration::from_secs(1));
                    println!("obligation: {}", obligation);
                    // continue;
                    let authority = Keypair::from_bytes(&wallet).unwrap();
                    let magik_client = client.program(magik_program);
                    let rs = magik_client
                        .request()
                        .accounts(magik_program::accounts::LendingCrank {
                            vault,
                            port_program,
                            reserve,
                            reserve_liquidity_supply,
                            reserve_collateral_mint,
                            source_liquidity,
                            lending_market,
                            transfer_authority,
                            destination_collateral, // maybe colle
                            lending_market_authority: lending_market_authority_pubkey, // maybe PDA
                            token_program: spl_token::ID,
                            clock: sysvar::clock::ID,
                        })
                        .args(magik_program::instruction::LendingCrank {
                            port_program_id: port_program,
                        })
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
            let nonce = Keypair::new().pubkey();
            let (obligation, ob_bump) = Pubkey::find_program_address(
                &[b"obligation", nonce.as_ref(), vault.as_ref()],
                &magik_program,
            );
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
    Ok(())
}
