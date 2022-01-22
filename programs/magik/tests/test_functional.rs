#![cfg(feature = "test-bpf")]

use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use magik_program;
use solana_program::system_instruction;
use solana_program::system_program;
use solana_program::sysvar;
use solana_sdk::signature::Keypair;
use spl_associated_token_account;
use {
    solana_program::{instruction::Instruction, pubkey::Pubkey},
    solana_program_test::*,
    std::str::FromStr,
};

mod helper;
use helper::{initialize_mint, mint_to, process_ins};

use solana_sdk::signature::Signer;

const INIT_AMOUNT: u64 = 1_000_000;
async fn init_user_token(
    banks_client: &mut BanksClient,
    user_keypair: &Keypair,
    token_keypair: &Keypair,
    payer_keypair: &Keypair,
) -> Pubkey {
    initialize_mint(
        banks_client,
        &payer_keypair,
        &token_keypair,
        &payer_keypair.pubkey(),
        6,
    )
    .await;

    process_ins(
        banks_client,
        &[
            spl_associated_token_account::create_associated_token_account(
                &payer_keypair.pubkey(),
                &user_keypair.pubkey(),
                &token_keypair.pubkey(),
            ),
        ],
        &payer_keypair,
        &[],
    )
    .await
    .ok()
    .unwrap_or_else(|| panic!("Can not create ATA account"));

    let user_ata = spl_associated_token_account::get_associated_token_address(
        &user_keypair.pubkey(),
        &token_keypair.pubkey(),
    );

    mint_to(
        payer_keypair,
        &token_keypair.pubkey(),
        &user_ata,
        INIT_AMOUNT,
        banks_client,
    )
    .await;

    user_ata
}

async fn init_user_synth_token(
    banks_client: &mut BanksClient,
    synth_mint: Pubkey,
    user_keypair: &Keypair,
    token_keypair: &Keypair,
    payer_keypair: &Keypair,
) -> Pubkey {
    process_ins(
        banks_client,
        &[
            spl_associated_token_account::create_associated_token_account(
                &payer_keypair.pubkey(),
                &user_keypair.pubkey(),
                &synth_mint,
            ),
        ],
        &payer_keypair,
        &[],
    )
    .await
    .ok()
    .unwrap_or_else(|| panic!("Can not create ATA account"));
    let user_vault_ata = spl_associated_token_account::get_associated_token_address(
        &user_keypair.pubkey(),
        &synth_mint,
    );

    user_vault_ata
}

// #[tokio::test]
// async fn test_init_port() {
//     port_variable_rate_lending_instructions::id()
//     let mut test = ProgramTest::new(
//         "port_finance_variable_rate_lending",
//         port_finance_variable_rate_lending::id(),
//         processor!(process_instruction),
//     );
// }

#[tokio::test]
async fn test_init() {
    let program_id = Pubkey::from_str("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS").unwrap();
    let program_test = ProgramTest::new(
        "magik_program",
        program_id,
        processor!(magik_program::entry),
    );

    let (mut banks_client, payer_keypair, _) = program_test.start().await;

    // Init user and token
    let user_keypair = Keypair::new();
    let token_keypair = Keypair::new();
    let mint_token = token_keypair.pubkey();
    // Fund user
    helper::process_and_assert_ok(
        &[system_instruction::transfer(
            &payer_keypair.pubkey(),
            &user_keypair.pubkey(),
            10_000_000_000,
        )],
        &payer_keypair,
        &[],
        &mut banks_client,
    )
    .await;
    let (vault, vault_bump) = Pubkey::find_program_address(
        &[
            b"vault",
            mint_token.as_ref(),
            payer_keypair.pubkey().as_ref(),
        ],
        &program_id,
    );
    let (vault_token, token_bump) = Pubkey::find_program_address(
        &[b"vault_token", mint_token.as_ref(), vault.as_ref()],
        &program_id,
    );
    let (synth_mint, mint_bump) = Pubkey::find_program_address(
        &[b"synth_mint", mint_token.as_ref(), vault.as_ref()],
        &program_id,
    );

    let user_ata = init_user_token(
        &mut banks_client,
        &user_keypair,
        &token_keypair,
        &payer_keypair,
    )
    .await;

    let lending_program = Pubkey::new_unique();
    let lending_market = Pubkey::new_unique();
    process_ins(
        &mut banks_client,
        &[Instruction {
            program_id,
            data: magik_program::instruction::Init {
                param: magik_program::state::InitParam {
                    bump: magik_program::state::Bump {
                        mint_bump,
                        token_bump,
                        vault_bump,
                    },
                    init_obligation: false,
                    percent: 50,
                },
                ob_bump: 1,
                nonce: Pubkey::default(),
            }
            .data(),
            accounts: magik_program::accounts::Init {
                vault,
                vault_token,
                mint_token,
                synth_mint,
                lending_program,
                authority: payer_keypair.pubkey(),
                obligation: vault,
                lending_market,
                rent: sysvar::rent::ID,
                system_program: system_program::id(),
                clock: sysvar::clock::ID,
                token_program: spl_token::id(),
            }
            .to_account_metas(None),
        }],
        &payer_keypair,
        &[&payer_keypair],
    )
    .await
    .ok()
    .unwrap_or_else(|| panic!("Can not Init "));

    let user_synth = init_user_synth_token(
        &mut banks_client,
        synth_mint,
        &user_keypair,
        &token_keypair,
        &payer_keypair,
    )
    .await;

    let (treasure, treasure_bump) = Pubkey::find_program_address(
        &[b"treasure", vault.as_ref(), user_keypair.pubkey().as_ref()],
        &program_id,
    );
    process_ins(
        &mut banks_client,
        &[Instruction {
            program_id,
            data: magik_program::instruction::Deposit {
                bump: treasure_bump,
                amount: 5000,
            }
            .data(),
            accounts: magik_program::accounts::Deposit {
                vault,
                vault_token,
                user_token: user_ata,
                owner: user_keypair.pubkey(),
                user_synth,
                treasure,
                rent: sysvar::rent::ID,
                system_program: system_program::id(),
                token_program: spl_token::id(),
            }
            .to_account_metas(None),
        }],
        &payer_keypair,
        &[&user_keypair],
    )
    .await
    .ok()
    .unwrap_or_else(|| panic!("Can not Deposit"));

    let mut borrow_amount = 1000;
    process_ins(
        &mut banks_client,
        &[Instruction {
            program_id,
            data: magik_program::instruction::Borrow {
                bump: treasure_bump,
                amount: borrow_amount,
            }
            .data(),
            accounts: magik_program::accounts::Borrow {
                vault,
                vault_token,
                synth_mint,
                owner: user_keypair.pubkey(),
                user_synth,
                treasure,
                system_program: system_program::id(),
                token_program: spl_token::id(),
            }
            .to_account_metas(None),
        }],
        &payer_keypair,
        &[&user_keypair],
    )
    .await
    .ok()
    .unwrap_or_else(|| panic!("Can not Borrow"));
    helper::verify_token_amount(synth_mint, user_synth, borrow_amount, &mut banks_client).await;

    borrow_amount = 3000;
    let isErr = process_ins(
        &mut banks_client,
        &[Instruction {
            program_id,
            data: magik_program::instruction::Borrow {
                bump: treasure_bump,
                amount: borrow_amount,
            }
            .data(),
            accounts: magik_program::accounts::Borrow {
                vault,
                vault_token,
                synth_mint,
                owner: user_keypair.pubkey(),
                user_synth,
                treasure,
                system_program: system_program::id(),
                token_program: spl_token::id(),
            }
            .to_account_metas(None),
        }],
        &payer_keypair,
        &[&user_keypair],
    )
    .await
    .is_err();
    assert_eq!(isErr, true);

    helper::verify_token_amount(synth_mint, user_synth, 1000, &mut banks_client).await
}
