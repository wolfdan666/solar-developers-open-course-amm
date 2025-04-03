use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token::{Mint, Token, TokenAccount}};

use crate::state::Pool;

#[derive(Accounts)]
#[instruction(fee: u16)]
pub struct Initialize<'info> {
    #[account(mut)]
    signer: Signer<'info>,
    mint_a: Account<'info, Mint>,
    mint_b: Account<'info, Mint>,
    #[account(
        init,
        payer = signer,
        mint::decimals = 0,
        mint::authority = pool,
        seeds = [b"lp", pool.key().as_ref()],
        bump
    )]
    mint_lp: Account<'info, Mint>,
    #[account(
        init,
        payer = signer,
        associated_token::authority = pool,
        associated_token::mint = mint_a
    )]
    pool_ata_a: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = signer,
        associated_token::authority = pool,
        associated_token::mint = mint_b
    )]
    pool_ata_b: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = signer,
        space = Pool::DISCRIMINATOR.len() + Pool::INIT_SPACE,
        seeds = [b"pool", mint_a.key().as_ref(), mint_b.key().as_ref(), fee.to_le_bytes().as_ref()],
        bump
    )]
    pool: Account<'info, Pool>,
    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
    system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(&mut self, fee: u16, bump: u8, lp_bump: u8) -> Result<()> {
        self.pool.set_inner(Pool {
            mint_a: self.mint_a.key(),
            mint_b: self.mint_a.key(),
            fee,
            bump,
            lp_bump,
        });
        Ok(())
    }
}
