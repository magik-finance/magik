
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{pubkey::Pubkey, system_program};
use anchor_spl::token::{self, Mint, TokenAccount};
use std::mem::size_of;

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
        init,
        seeds = [b"vault", mint_token.key().as_ref(), authority.key().as_ref()],
        bump = param.bump.vault_bump,
        payer = authority,
        space = size_of::<Vault>() + 8,
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        init,
        seeds = [b"vault_token", mint_token.key().as_ref(), vault.key().as_ref()],
        bump = param.bump.token_bump,
        token::mint = mint_token,
        token::authority = vault,
        payer = authority,
    )]
    pub vault_token: Account<'info, TokenAccount>,

    #[account(
        init, 
        seeds = [b"synth_token", mint_token.key().as_ref(), vault.key().as_ref()],
        bump = param.bump.mint_bump,
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
    pub total_deposit: u64,
}

#[account]
pub struct Treasure {
    pub current_deposit: u64,
    pub current_borrow: u64,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct Deposit<'info> {
    #[account(
        init_if_needed,
        seeds = [b"treasure", vault.key().as_ref(), owner.key().as_ref()],
        bump = bump,
        payer = owner,
        space = size_of::<Treasure>() + 8,
    )]
    pub treasure: Account<'info, Treasure>,

    #[account(mut, has_one = owner)]
    pub user_token: Account<'info, TokenAccount>,

    #[account(mut, constraint = vault.mint_token == user_token.mint)]
    pub vault: Account<'info, Vault>,

    #[account(mut, constraint = vault_token.mint == vault.mint_token)]
    pub vault_token: Account<'info, TokenAccount>,

    #[account(mut)]
    pub vault_mint: Account<'info, Mint>,

    #[account(mut, constraint = user_vault.mint == vault.synth_token)]
    pub user_vault: Account<'info, TokenAccount>,

    #[account(signer)]
    pub owner: AccountInfo<'info>,

    #[account(address = spl_token::ID)]
    pub token_program: AccountInfo<'info>,

    #[account(address = system_program::ID)]
    pub system_program: AccountInfo<'info>,

    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct Borrow<'info> {
    #[account(
        init_if_needed,
        seeds = [b"treasure", vault.key().as_ref(), owner.key().as_ref()],
        bump = bump,
        payer = owner,
        space = size_of::<Treasure>() + 8,
    )]
    pub treasure: Account<'info, Treasure>,

    #[account(mut, has_one = owner)]
    pub user_token: Account<'info, TokenAccount>,

    #[account(mut, constraint = vault.mint_token == user_token.mint)]
    pub vault: Account<'info, Vault>,

    #[account(mut, constraint = vault_token.mint == vault.mint_token)]
    pub vault_token: Account<'info, TokenAccount>,

    #[account(mut)]
    pub vault_mint: Account<'info, Mint>,

    #[account(mut, constraint = user_vault.mint == vault.synth_token)]
    pub user_vault: Account<'info, TokenAccount>,

    #[account(signer)]
    pub owner: AccountInfo<'info>,

    #[account(address = system_program::ID)]
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