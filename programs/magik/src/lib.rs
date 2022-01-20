mod parameters;

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    system_program,
    pubkey::Pubkey,
};
use anchor_spl::token::{self, Mint, TokenAccount, Transfer, MintTo, Burn};
use std::mem::size_of;

use port_anchor_adaptor::InitObligation;

use crate::{parameters::Parameters};
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
#[program]
pub mod magik {
    use super::*;
    pub fn init(ctx: Context<Init>, bump: Bump, percent: u64) -> ProgramResult {
        Parameters::verify_percent(percent);
        {
            let ref mut vault = ctx.accounts.vault;
            vault.bump = bump.vault_bump;
            vault.mint_token = ctx.accounts.mint_token.key();
            vault.vault_token = ctx.accounts.vault_token.key();
            vault.synth_token = ctx.accounts.synth_token.key();
            vault.payer = ctx.accounts.authority.key();

            vault.percent = percent;

            emit!(InitVault {
                mint_token: vault.mint_token,
                vault_token: vault.vault_token,
                synth_token: vault.synth_token,
                payer : vault.payer,
                percent: vault.percent,
            });
        }

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
        let init_obligation_ctx = CpiContext::new_with_signer(port_program, cpi_account, signer_seeds);

        port_anchor_adaptor::init_obligation(init_obligation_ctx)?;

        Ok(())
    }


    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> ProgramResult {
        //User mint synthSTBL up to 50% of they STBL position
        let cpi_accounts = Transfer {
            from: ctx.accounts.depositor.to_account_info().clone(),
            to: ctx.accounts.vault_token.to_account_info().clone(),
            authority: ctx.accounts.owner.clone(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        //User mint synthSTBL up to 50% of they STBL position
        let cpi_program = ctx.accounts.token_program.clone();
        let  signer_seeds = &[
            b"vault".as_ref(), 
            ctx.accounts.vault.mint_token.as_ref(),
            ctx.accounts.vault.payer.as_ref(),
            &[ctx.accounts.vault.bump],
        ];
        let signer = &[&signer_seeds[..]];
        let mint_to_ctx = CpiContext::new_with_signer(
                cpi_program,
                MintTo {
                mint: ctx.accounts.vault_mint.to_account_info().clone(),
                to:  ctx.accounts.user_vault.to_account_info().clone(),
                authority: ctx.accounts.vault.to_account_info().clone(),
                }, signer);
        
        let mint_amount = amount * ctx.accounts.vault.percent / 100;
        token::mint_to(mint_to_ctx, mint_amount)?;

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

        let cpi_account = port_anchor_adaptor::Deposit{
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
        let init_obligation_ctx = CpiContext::new_with_signer(port_program, cpi_account, signer_seeds);

        port_anchor_adaptor::deposit_reserve(init_obligation_ctx, amount)?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct LendingCrank<'info> {
    pub vault: Account<'info, Vault>,
    
    pub port_program: AccountInfo<'info>,

    pub source_liquidity: AccountInfo<'info>,
    pub destination_collateral: AccountInfo<'info>,
    pub reserve: AccountInfo<'info>,
    pub reserve_liquidity_supply: AccountInfo<'info>,
    pub reserve_collateral_mint: AccountInfo<'info>,
    pub lending_market: AccountInfo<'info>,
    pub lending_market_authority: AccountInfo<'info>,
    pub transfer_authority: AccountInfo<'info>,
    pub clock: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct Bump {
    pub vault_bump: u8,
    pub token_bump: u8,
    pub mint_bump: u8,
}

#[derive(Accounts)]
#[instruction(bump: Bump)]
pub struct Init<'info> {
    // For each token we have one vault
    #[account(
        init,
        seeds = [b"vault", mint_token.key().as_ref(), authority.key().as_ref()],
        bump = bump.vault_bump,
        payer = authority,
        space = size_of::<Vault>() + 8,
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        init,
        seeds = [b"vault_token", mint_token.key().as_ref(), vault.key().as_ref()],
        bump = bump.token_bump,
        token::mint = mint_token,
        token::authority = vault,
        payer = authority,
    )]
    pub vault_token: Account<'info, TokenAccount>,

    #[account(
        init, 
        seeds = [b"synth_token", mint_token.key().as_ref(), vault.key().as_ref()],
        bump = bump.mint_bump,
        mint::authority = vault,
        mint::decimals = mint_token.decimals,
        payer = authority,
    )]
    pub synth_token: Account<'info, Mint>,

    pub mint_token: Account<'info, Mint>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub obligation: AccountInfo<'info>,
    pub lending_market: AccountInfo<'info>,

    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,

    pub port_program: AccountInfo<'info>,

    #[account(address = system_program::ID)]
    pub system_program: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
}

#[account]
pub struct Vault {
    pub bump: u8,
    pub payer: Pubkey,
    pub mint_token: Pubkey,  // The token this vault keep
    pub vault_token: Pubkey, // PDA for this vault keep the token
    pub synth_token: Pubkey,  // LP token mint
    pub percent: u64,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut, has_one = owner)]
    depositor: Account<'info, TokenAccount>,

    #[account(mut, constraint = vault.mint_token == depositor.mint)]
    vault: Account<'info, Vault>,

    #[account(mut, constraint = vault_token.mint == vault.mint_token)]
    vault_token: Account<'info, TokenAccount>,

    #[account(mut)]
    vault_mint: Account<'info, Mint>,

    #[account(mut, constraint = user_vault.mint == vault.synth_token)]
    user_vault: Account<'info, TokenAccount>,

    #[account(signer)]
    owner: AccountInfo<'info>,

    token_program: AccountInfo<'info>,
}

#[event]
pub struct InitVault {
    pub payer: Pubkey,
    pub mint_token: Pubkey,  // The token this vault keep
    pub vault_token: Pubkey, // PDA for this vault keep the token
    pub synth_token: Pubkey,  
    pub percent: u64,
}