pub fn deposit_reserve<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, PortDeposit<'info>>,
    amount: u64,
    port_program: Pubkey,
) -> ProgramResult {
    let ix = deposit_reserve_liquidity(
        port_program,
        amount,
        ctx.accounts.source_liquidity.key(),
        ctx.accounts.destination_collateral.key(),
        ctx.accounts.reserve.key(),
        ctx.accounts.reserve_liquidity_supply.key(),
        ctx.accounts.reserve_collateral_mint.key(),
        ctx.accounts.lending_market.key(),
        ctx.accounts.lending_market_authority.key(),
        ctx.accounts.transfer_authority.key(),
    );

    invoke_signed(
        &ix,
        &[
            ctx.accounts.source_liquidity,
            ctx.accounts.destination_collateral,
            ctx.accounts.reserve,
            ctx.accounts.reserve_liquidity_supply,
            ctx.accounts.reserve_collateral_mint,
            ctx.accounts.lending_market,
            ctx.accounts.lending_market_authority,
            ctx.accounts.transfer_authority,
            ctx.accounts.clock,
            ctx.accounts.token_program,
            ctx.program,
        ],
        ctx.signer_seeds,
    )
}
/// Number of bytes in a pubkey
pub const PUBKEY_BYTES: usize = 32;
/// Creates a 'DepositReserveLiquidity' instruction.
#[allow(clippy::too_many_arguments)]
pub fn deposit_reserve_liquidity(
    program_id: Pubkey,
    liquidity_amount: u64,
    source_liquidity_pubkey: Pubkey,
    destination_collateral_pubkey: Pubkey,
    reserve_pubkey: Pubkey,
    reserve_liquidity_supply_pubkey: Pubkey,
    reserve_collateral_mint_pubkey: Pubkey,
    lending_market_pubkey: Pubkey,
    lending_market_authority_pubkey: Pubkey,
    user_transfer_authority_pubkey: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(source_liquidity_pubkey, false),
            AccountMeta::new(destination_collateral_pubkey, false),
            AccountMeta::new(reserve_pubkey, false),
            AccountMeta::new(reserve_liquidity_supply_pubkey, false),
            AccountMeta::new(reserve_collateral_mint_pubkey, false),
            AccountMeta::new_readonly(lending_market_pubkey, false),
            AccountMeta::new_readonly(lending_market_authority_pubkey, false),
            AccountMeta::new_readonly(user_transfer_authority_pubkey, true),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: LendingInstruction::DepositReserveLiquidity { liquidity_amount }.pack(),
    }
}

#[derive(Accounts)]
pub struct PortDeposit<'info> {
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

#[error]
pub enum VaultError {
    #[msg("Exceed Borrow Amount")]
    ExceedBorrowAmount,
}

pub fn init_obligation<'a, 'b, 'c, 'info>(
    lending_program: &Pubkey,
    ctx: CpiContext<'a, 'b, 'c, 'info, InitObligation<'info>>,
) -> ProgramResult {
    let ix = Instruction {
        program_id: *lending_program,
        accounts: vec![
            AccountMeta::new(ctx.accounts.obligation.key(), false),
            AccountMeta::new_readonly(ctx.accounts.lending_market.key(), false),
            AccountMeta::new_readonly(ctx.accounts.obligation_owner.key(), true),
            AccountMeta::new_readonly(ctx.accounts.clock.key(), false),
            AccountMeta::new_readonly(ctx.accounts.rent.key(), false),
            AccountMeta::new_readonly(ctx.accounts.spl_token_id.key(), false),
        ],
        data: LendingInstruction::InitObligation.pack(),
    };

    invoke_signed(
        &ix,
        &[
            ctx.accounts.obligation,
            ctx.accounts.lending_market,
            ctx.accounts.obligation_owner,
            ctx.accounts.clock,
            ctx.accounts.rent,
            ctx.accounts.spl_token_id,
            ctx.program,
        ],
        ctx.signer_seeds,
    )
}

#[derive(Accounts)]
pub struct InitObligation<'info> {
    pub obligation: AccountInfo<'info>,
    pub lending_market: AccountInfo<'info>,
    pub obligation_owner: AccountInfo<'info>,
    pub clock: AccountInfo<'info>,
    pub rent: AccountInfo<'info>,
    pub spl_token_id: AccountInfo<'info>,
}

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{pubkey::Pubkey, system_program, sysvar};
use anchor_spl::token::{self, Burn, Mint, MintTo, TokenAccount, Transfer};
use port_variable_rate_lending_instructions::instruction::{
    redeem_reserve_collateral, LendingInstruction,
};
use solana_program::instruction::Instruction;
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_option::COption;

#[derive(Accounts)]
pub struct RefreshReserve<'info> {
    pub reserve: AccountInfo<'info>,
    pub clock: AccountInfo<'info>,
    pub oracle: AccountInfo<'info>,
}

pub fn refresh_port_reserve<'a, 'b, 'c, 'info>(
    program_id: Pubkey,
    ctx: CpiContext<'a, 'b, 'c, 'info, RefreshReserve<'info>>,
) -> ProgramResult {
    let oracle = ctx.remaining_accounts;
    let ix = refresh_reserve(
        program_id,
        ctx.accounts.reserve.key(),
        oracle
            .first()
            .map_or(COption::None, |k| COption::Some(k.key())),
    );
    let mut accounts = vec![ctx.accounts.reserve, ctx.accounts.clock, ctx.program];
    accounts.extend(oracle.into_iter().next());
    invoke(&ix, &accounts)
}
/// Creates a `RefreshReserve` instruction
pub fn refresh_reserve(
    program_id: Pubkey,
    reserve_pubkey: Pubkey,
    reserve_liquidity_oracle_pubkey: COption<Pubkey>,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(reserve_pubkey, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];
    if let COption::Some(reserve_liquidity_oracle_pubkey) = reserve_liquidity_oracle_pubkey {
        accounts.push(AccountMeta::new_readonly(
            reserve_liquidity_oracle_pubkey,
            false,
        ));
    }
    Instruction {
        program_id,
        accounts,
        data: LendingInstruction::RefreshReserve.pack(),
    }
}

pub fn redeem<'a, 'b, 'c, 'info>(
    program_id: Pubkey,
    ctx: CpiContext<'a, 'b, 'c, 'info, PortRedeem<'info>>,
    amount: u64,
) -> ProgramResult {
    let ix = redeem_reserve_collateral(
        program_id,
        amount,
        ctx.accounts.source_collateral.key(),
        ctx.accounts.destination_liquidity.key(),
        ctx.accounts.reserve.key(),
        ctx.accounts.reserve_collateral_mint.key(),
        ctx.accounts.reserve_liquidity_supply.key(),
        ctx.accounts.lending_market.key(),
        ctx.accounts.transfer_authority.key(),
    );

    invoke_signed(
        &ix,
        &[
            ctx.accounts.source_collateral,
            ctx.accounts.destination_liquidity,
            ctx.accounts.reserve,
            ctx.accounts.reserve_collateral_mint,
            ctx.accounts.reserve_liquidity_supply,
            ctx.accounts.lending_market,
            ctx.accounts.lending_market_authority,
            ctx.accounts.transfer_authority,
            ctx.accounts.clock,
            ctx.accounts.token_program,
            ctx.program,
        ],
        ctx.signer_seeds,
    )
}

#[derive(Accounts)]
pub struct PortRedeem<'info> {
    pub source_collateral: AccountInfo<'info>,
    pub destination_liquidity: AccountInfo<'info>,
    pub reserve: AccountInfo<'info>,
    pub reserve_collateral_mint: AccountInfo<'info>,
    pub reserve_liquidity_supply: AccountInfo<'info>,
    pub lending_market: AccountInfo<'info>,
    pub lending_market_authority: AccountInfo<'info>,
    pub transfer_authority: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub clock: AccountInfo<'info>,
}
