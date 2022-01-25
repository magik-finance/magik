#![allow(unused)]
mod parameters;
pub mod port;
pub mod state;

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{pubkey::Pubkey, system_program, sysvar};
use anchor_spl::token::{self, Burn, Mint, MintTo, TokenAccount, Transfer};
use port::*;
use port_variable_rate_lending_instructions::instruction::LendingInstruction;
use solana_program::instruction::Instruction;
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_option::COption;
use state::*;

use crate::parameters::Parameters;
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
#[program]
pub mod magik {
    use solana_program::{program::invoke_signed, system_instruction::create_account};

    use super::*;
    pub fn init(ctx: Context<Init>, param: InitParam, nonce: Pubkey, ob_bump: u8) -> ProgramResult {
        msg!("Init params {:?}", param);
        Parameters::verify_percent(param.percent);
        {
            let ref mut vault = ctx.accounts.vault;
            vault.bump = param.bump.vault_bump;
            vault.mint_token = ctx.accounts.mint_token.key();
            vault.vault_token = ctx.accounts.vault_token.key();
            vault.synth_token = ctx.accounts.synth_mint.key();
            vault.payer = ctx.accounts.authority.key();

            vault.percent = param.percent;

            emit!(InitVault {
                mint_token: vault.mint_token,
                vault_token: vault.vault_token,
                synth_token: vault.synth_token,
                payer: vault.payer,
                percent: vault.percent,
            });
        }

        if param.init_obligation {
            let ref vault = ctx.accounts.vault;
            let vault_key = ctx.accounts.vault.clone().key();
            let cpi_account = InitObligation {
                clock: ctx.accounts.clock.to_account_info(),
                lending_market: ctx.accounts.lending_market.to_account_info(),
                obligation: ctx.accounts.obligation.to_account_info(),
                obligation_owner: ctx.accounts.vault.clone().to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
                spl_token_id: ctx.accounts.token_program.to_account_info(),
            };

            let lending_program = ctx.accounts.lending_program.to_account_info();

            let lending_program_id = lending_program.key;
            let seeds = &[
                b"obligation".as_ref(),
                nonce.as_ref(),
                vault_key.as_ref(),
                &[ob_bump],
            ];
            let signers_seeds = &[&seeds[..]];
            invoke_signed(
                &create_account(
                    &ctx.accounts.authority.key,
                    &ctx.accounts.obligation.key,
                    7266240,
                    916,
                    lending_program.key,
                ),
                &[
                    ctx.accounts.authority.to_account_info(),
                    ctx.accounts.obligation.to_account_info(),
                ],
                signers_seeds,
            )?;

            let vault_seeds = &[
                b"vault".as_ref(),
                ctx.accounts.vault.mint_token.as_ref(),
                ctx.accounts.vault.payer.as_ref(),
                &[ctx.accounts.vault.bump],
            ];
            let vault_signer_seeds = &[&vault_seeds[..]];

            let init_obligation_ctx =
                CpiContext::new_with_signer(lending_program, cpi_account, vault_signer_seeds);

            init_obligation(lending_program_id, init_obligation_ctx)?;
        }

        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, bump: u8, amount: u64) -> ProgramResult {
        msg!("Deposit {}", amount);
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token.to_account_info().clone(),
            to: ctx.accounts.vault_token.to_account_info().clone(),
            authority: ctx.accounts.owner.clone(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        let ref mut vault = ctx.accounts.vault;
        vault.total_deposit += amount;

        let ref mut depositor = ctx.accounts.treasure;
        depositor.current_deposit += amount;

        Ok(())
    }

    pub fn borrow(ctx: Context<Borrow>, bump: u8, amount: u64) -> ProgramResult {
        msg!("Borrow {}", amount);
        let ref mut treasure = ctx.accounts.treasure;
        let ref vault = ctx.accounts.vault;
        let total_borrow = treasure.current_borrow + amount;
        msg!("Current {} total {}", treasure.current_borrow, total_borrow);
        if total_borrow / vault.percent * 100 > treasure.current_deposit {
            return Err(VaultError::ExceedBorrowAmount.into());
        }

        // User mint synthSTBL up to 50% of they STBL position
        let cpi_program = ctx.accounts.token_program.clone();
        let seeds = &[
            b"vault".as_ref(),
            ctx.accounts.vault.mint_token.as_ref(),
            ctx.accounts.vault.payer.as_ref(),
            &[ctx.accounts.vault.bump],
        ];
        let signer_seeds = &[&seeds[..]];
        let mint_to_ctx = CpiContext::new_with_signer(
            cpi_program,
            MintTo {
                mint: ctx.accounts.synth_mint.to_account_info().clone(),
                to: ctx.accounts.user_synth.to_account_info().clone(),
                authority: ctx.accounts.vault.to_account_info().clone(),
            },
            signer_seeds,
        );

        token::mint_to(mint_to_ctx, amount)?;

        treasure.current_borrow += amount;

        Ok(())
    }

    pub fn lending_crank(ctx: Context<LendingCrank>, lending_amount: u64) -> ProgramResult {
        let ref mut vault = ctx.accounts.vault;

        let port_program = ctx.accounts.port_program.to_account_info();
        let seeds = &[
            b"vault".as_ref(),
            vault.mint_token.as_ref(),
            vault.payer.as_ref(),
            &[vault.bump],
        ];
        let port_program_id = port_program.key();
        msg!("Borrow {}", port_program_id);

        let cpi_account = PortDeposit {
            destination_collateral: ctx.accounts.destination_collateral.to_account_info(),
            lending_market: ctx.accounts.lending_market.to_account_info(),
            lending_market_authority: ctx.accounts.lending_market_authority.to_account_info(),
            reserve: ctx.accounts.reserve.to_account_info(),
            reserve_collateral_mint: ctx.accounts.reserve_collateral_mint.to_account_info(),
            reserve_liquidity_supply: ctx.accounts.reserve_liquidity_supply.to_account_info(),
            source_liquidity: ctx.accounts.source_liquidity.to_account_info(),
            transfer_authority: ctx.accounts.transfer_authority.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
            clock: ctx.accounts.clock.to_account_info(),
        };

        let signer_seeds = &[&seeds[..]];
        let init_obligation_ctx =
            CpiContext::new_with_signer(port_program.clone(), cpi_account, signer_seeds);

        deposit_reserve(init_obligation_ctx, lending_amount, port_program_id)?;
        Ok(())
    }

    pub fn redeem_crank(ctx: Context<RedeemCrank>, redeem_amount: u64) -> ProgramResult {
        let ref mut vault = ctx.accounts.vault;

        let port_program = ctx.accounts.port_program.to_account_info();
        let seeds = &[
            b"vault".as_ref(),
            vault.mint_token.as_ref(),
            vault.payer.as_ref(),
            &[vault.bump],
        ];
        let port_program_id = port_program.key();
        msg!("redeem_crank {}", port_program_id);

        let cpi_account = PortRedeem {
            destination_liquidity: ctx.accounts.destination_liquidity.to_account_info(),
            lending_market: ctx.accounts.lending_market.to_account_info(),
            lending_market_authority: ctx.accounts.lending_market_authority.to_account_info(),
            reserve: ctx.accounts.reserve.to_account_info(),
            reserve_collateral_mint: ctx.accounts.reserve_collateral_mint.to_account_info(),
            reserve_liquidity_supply: ctx.accounts.reserve_liquidity_supply.to_account_info(),
            source_collateral: ctx.accounts.source_collateral.to_account_info(),
            transfer_authority: ctx.accounts.transfer_authority.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
            clock: ctx.accounts.clock.to_account_info(),
        };

        let signer_seeds = &[&seeds[..]];
        let ctx = CpiContext::new_with_signer(port_program.clone(), cpi_account, signer_seeds);

        redeem(port_program_id, ctx, redeem_amount)?;

        Ok(())
    }
}
