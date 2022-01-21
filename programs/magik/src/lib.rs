mod parameters;
pub mod state;

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{pubkey::Pubkey, system_program};
use anchor_spl::token::{self, Burn, Mint, MintTo, TokenAccount, Transfer};
use state::*;

use port_anchor_adaptor::InitObligation;

use crate::parameters::Parameters;
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
#[program]
pub mod magik {
    use super::*;
    pub fn init(ctx: Context<Init>, param: InitParam) -> ProgramResult {
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
            let cpi_account = InitObligation {
                clock: ctx.accounts.clock.to_account_info(),
                lending_market: ctx.accounts.lending_market.to_account_info(),
                obligation: ctx.accounts.obligation.to_account_info(),
                obligation_owner: ctx.accounts.vault.clone().to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
                spl_token_id: ctx.accounts.token_program.to_account_info(),
            };

            let port_program = ctx.accounts.port_program.to_account_info();

            let seeds = &[
                b"vault".as_ref(),
                vault.mint_token.as_ref(),
                vault.payer.as_ref(),
                &[vault.bump],
            ];

            let signer_seeds = &[&seeds[..]];
            let init_obligation_ctx =
                CpiContext::new_with_signer(port_program, cpi_account, signer_seeds);

            port_anchor_adaptor::init_obligation(init_obligation_ctx)?;
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

    pub fn lending_crank(ctx: Context<LendingCrank>, amount: u64) -> ProgramResult {
        let ref mut vault = ctx.accounts.vault;

        let port_program = ctx.accounts.port_program.to_account_info();
        let seeds = &[
            b"vault".as_ref(),
            vault.mint_token.as_ref(),
            vault.payer.as_ref(),
            &[vault.bump],
        ];

        let cpi_account = port_anchor_adaptor::Deposit {
            clock: ctx.accounts.clock.to_account_info(),
            destination_collateral: ctx.accounts.destination_collateral.to_account_info(),
            lending_market: ctx.accounts.lending_market.to_account_info(),
            lending_market_authority: ctx.accounts.lending_market_authority.to_account_info(),
            reserve: ctx.accounts.reserve.to_account_info(),
            reserve_collateral_mint: ctx.accounts.reserve_collateral_mint.to_account_info(),
            reserve_liquidity_supply: ctx.accounts.reserve_liquidity_supply.to_account_info(),
            source_liquidity: ctx.accounts.source_liquidity.to_account_info(),
            transfer_authority: ctx.accounts.transfer_authority.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
        };

        let signer_seeds = &[&seeds[..]];
        let init_obligation_ctx =
            CpiContext::new_with_signer(port_program, cpi_account, signer_seeds);

        port_anchor_adaptor::deposit_reserve(init_obligation_ctx, amount)?;
        Ok(())
    }
}

#[error]
pub enum VaultError {
    #[msg("Exceed Borrow Amount")]
    ExceedBorrowAmount,
}
