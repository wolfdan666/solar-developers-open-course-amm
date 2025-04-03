use anchor_lang::prelude::*;

pub mod state;

pub mod context;
pub use context::*;

declare_id!("2wmWHXHy6F3Yz2CaaWNp7Bfgz4c4kpPXuHiDyVajZARu");

#[program]
pub mod amm {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, fee: u16) -> Result<()> {
        ctx.accounts.initialize(fee, ctx.bumps.pool, ctx.bumps.mint_lp)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64, max_token_a: u64, max_token_b: u64) -> Result<()> {
        ctx.accounts.deposit(amount, max_token_a, max_token_b)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64, min_token_a: u64, min_token_b: u64) -> Result<()> {
        ctx.accounts.withdraw(amount, min_token_a, min_token_b)
    }

    pub fn swap(ctx: Context<Swap>, amount: u64, max_amount_in: u64, is_a: bool) -> Result<()> {
        ctx.accounts.swap(amount, max_amount_in, is_a)
    }
}