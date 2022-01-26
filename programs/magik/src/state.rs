#![allow(unused)]
use anchor_lang::accounts::program_account::ProgramAccount;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{pubkey::Pubkey, system_program};
use anchor_spl::token::{self, Mint, TokenAccount};
use std::mem::size_of;
#[derive(Accounts)]
pub struct RedeemCrank<'info> {
    pub vault: ProgramAccount<'info, Vault>,
    pub port_program: UncheckedAccount<'info>,

    #[account(mut)]
    pub source_collateral: AccountInfo<'info>,

    #[account(mut)]
    pub destination_liquidity: AccountInfo<'info>,
    #[account(mut)]
    pub reserve: AccountInfo<'info>,

    #[account(mut)]
    pub reserve_collateral_mint: AccountInfo<'info>,

    #[account(mut)]
    pub reserve_liquidity_supply: AccountInfo<'info>,
    #[account(mut)]
    pub lending_market: AccountInfo<'info>,
    #[account(mut)]
    pub lending_market_authority: AccountInfo<'info>,
    #[account(mut)]
    pub transfer_authority: AccountInfo<'info>,

    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct LendingCrank<'info> {
    pub vault: ProgramAccount<'info, Vault>,
    pub port_program: UncheckedAccount<'info>,
    #[account(mut)]
    pub source_liquidity: UncheckedAccount<'info>,

    #[account(mut)]
    pub destination_collateral: UncheckedAccount<'info>,
    #[account(mut)]
    pub reserve: UncheckedAccount<'info>,
    #[account(mut)]
    pub reserve_liquidity_supply: UncheckedAccount<'info>,
    #[account(mut)]
    pub reserve_collateral_mint: UncheckedAccount<'info>,
    #[account(mut)]
    pub lending_market: UncheckedAccount<'info>,
    #[account(mut)]
    pub lending_market_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub transfer_authority: UncheckedAccount<'info>,

    // #[account(mut)]
    // pub oracle: UncheckedAccount<'info>,
    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,
    pub clock: Sysvar<'info, Clock>,
}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, Debug)]
pub struct Bump {
    pub vault_bump: u8,
    pub token_bump: u8,
    pub mint_bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, Debug)]
pub struct InitParam {
    pub bump: Bump,
    pub percent: u64,
    pub init_obligation: bool,
}
#[derive(Accounts)]
#[instruction(param: InitParam)]
pub struct Init<'info> {
    // For each token we have one vault
    #[account(
        init_if_needed,
        seeds = [b"vault", mint_token.key().as_ref(), authority.key().as_ref()],
        bump = param.bump.vault_bump,
        payer = authority,
        space = size_of::<Vault>() + 8,
    )]
    pub vault: ProgramAccount<'info, Vault>,

    #[account(
        init_if_needed,
        seeds = [b"vault_token", mint_token.key().as_ref(), vault.key().as_ref()],
        bump = param.bump.token_bump,
        token::mint = mint_token,
        token::authority = vault,
        payer = authority,
    )]
    pub vault_token: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        seeds = [b"synth_mint", mint_token.key().as_ref(), vault.key().as_ref()],
        bump = param.bump.mint_bump,
        mint::authority = vault,
        mint::decimals = mint_token.decimals,
        payer = authority,
    )]
    pub synth_mint: Account<'info, Mint>,
    pub mint_token: Account<'info, Mint>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub obligation: UncheckedAccount<'info>,

    pub lending_market: UncheckedAccount<'info>,
    pub lending_program: UncheckedAccount<'info>,

    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,

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
    pub synth_token: Pubkey, // LP token mint
    pub percent: u64,
    pub total_deposit: u64,

    // Total amount of yield harvested from the lending program
    pub total_yield_harvested: u64,
    //
}

#[account]
pub struct Treasure {
    // Currently deposited amount of tokens
    pub current_deposit: u64,

    // Currently outstanding amount of borrowed token
    pub current_borrow: u64,

    // Current amount of credit accumulated through yield in ecxess of outstanding debt repayments
    pub current_credit: u64,

    // Total amount of yield this user has utilized to repay debt / issue credit
    pub total_earned_yield: u64,

    // Total vault yield at the point of last account update
    // This is necessary to calculate the delta between how much yield the user has already been given their
    // share off vs how much new yield they have a claim to
    pub last_known_vault_yield: u64,
}

impl Treasure {
    // This function should be called every time a user interacts with the Vault
    // It calculates the earned yield and adjust debt and credit accordingly
    pub fn use_earned_yield(&mut self, vault: &Vault) -> u64 {
        let earned_yield =
            self.calculate_earned_yield(vault.total_yield_harvested, vault.total_deposit);

        // Reduce debt with earned yield
        let current_debt = self.current_borrow;
        let debt_after_auto_repay = current_debt as i128 - earned_yield as i128;
        if (debt_after_auto_repay < 0) {
            self.current_borrow = 0;
            self.current_credit += (debt_after_auto_repay.abs()) as u64;
        } else {
            self.current_borrow -= earned_yield;
        }

        self.last_known_vault_yield = vault.total_yield_harvested;
        self.total_earned_yield += earned_yield;

        earned_yield
    }

    pub fn draw_credit(&mut self, amount: u64) -> u64 {
        // Prioritise earned credit over issuing debt
        let remaining_credit = self.current_credit as i128 - amount as i128;

        // Amount is in excess of earned credit
        if (remaining_credit < 0) {
            self.current_credit = 0;
            self.current_borrow += (remaining_credit.abs()) as u64;
        // Amount is covered by earned credit
        } else {
            self.current_credit = remaining_credit as u64;
        }

        self.current_borrow
    }

    fn calculate_earned_yield(&self, vault_yield: u64, vault_total_deposit: u64) -> u64 {
        if (self.current_deposit == 0) {
            return 0;
        }

        let total_new_yield = vault_yield - self.last_known_vault_yield;
        let user_share = self.current_deposit as f64 / vault_total_deposit as f64;
        (user_share * self.current_deposit as f64) as u64
    }
}

#[derive(Accounts)]
#[instruction(bump: u8, amount: u64)]
pub struct Deposit<'info> {
    #[account(
        init_if_needed,
        seeds = [b"treasure", vault.key().as_ref(), owner.key().as_ref()],
        bump = bump,
        payer = owner,
        space = size_of::<Treasure>() + 8,
    )]
    pub treasure: ProgramAccount<'info, Treasure>,

    #[account(mut, has_one = owner)]
    pub user_token: Account<'info, TokenAccount>,

    #[account(mut, constraint = vault.mint_token == user_token.mint)]
    pub vault: ProgramAccount<'info, Vault>,

    #[account(mut, constraint = vault_token.mint == vault.mint_token)]
    pub vault_token: Account<'info, TokenAccount>,

    #[account(mut, constraint = user_synth.mint == vault.synth_token)]
    pub user_synth: Account<'info, TokenAccount>,

    #[account(mut, signer)]
    pub owner: AccountInfo<'info>,

    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,

    #[account(address = system_program::ID)]
    pub system_program: AccountInfo<'info>,

    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(bump: u8, amount: u64)]
pub struct Borrow<'info> {
    #[account(
        init_if_needed,
        seeds = [b"treasure", vault.key().as_ref(), owner.key().as_ref()],
        bump = bump,
        payer = owner,
        space = size_of::<Treasure>() + 8,
    )]
    pub treasure: Account<'info, Treasure>,

    #[account(mut)]
    pub vault: Account<'info, Vault>,

    #[account(mut, constraint = vault_token.mint == vault.mint_token)]
    pub vault_token: Account<'info, TokenAccount>,

    #[account(mut)]
    pub synth_mint: Account<'info, Mint>,

    #[account(mut, constraint = user_synth.mint == vault.synth_token)]
    pub user_synth: Account<'info, TokenAccount>,

    #[account(mut, signer)]
    pub owner: AccountInfo<'info>,

    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,

    #[account(address = system_program::ID)]
    pub system_program: AccountInfo<'info>,
}

#[event]
pub struct InitVault {
    pub payer: Pubkey,
    pub mint_token: Pubkey,  // The token this vault keep
    pub vault_token: Pubkey, // PDA for this vault keep the token
    pub synth_token: Pubkey,
    pub percent: u64,
}
