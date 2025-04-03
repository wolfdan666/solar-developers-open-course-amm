use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token::{burn, transfer, Burn, Mint, Token, TokenAccount, Transfer}};

use crate::state::Pool;

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    signer: Signer<'info>,
    mint_a: Account<'info, Mint>,
    mint_b: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [b"lp", pool.key().as_ref()],
        bump
    )]
    mint_lp: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::authority = signer,
        associated_token::mint = mint_a
    )]
    signer_ata_a: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::authority = signer,
        associated_token::mint = mint_b
    )]
    signer_ata_b: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::authority = signer,
        associated_token::mint = mint_lp
    )]
    signer_ata_lp: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::authority = pool,
        associated_token::mint = mint_a
    )]
    pool_ata_a: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::authority = pool,
        associated_token::mint = mint_b
    )]
    pool_ata_b: Account<'info, TokenAccount>,
    #[account(
        seeds = [b"pool", mint_a.key().as_ref(), mint_b.key().as_ref(), pool.fee.to_le_bytes().as_ref()],
        bump = pool.bump
    )]
    pool: Account<'info, Pool>,
    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
    system_program: Program<'info, System>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, amount: u64, min_token_a: u64, min_token_b: u64) -> Result<()> {
        let k = (self.pool_ata_a.amount as u128).checked_mul(self.pool_ata_b.amount.into()).ok_or(ProgramError::ArithmeticOverflow)?;
        let k2 = k.checked_sub(amount as u128).ok_or(ProgramError::ArithmeticOverflow)?;
        let ratio = k2.checked_mul(1000000).ok_or(ProgramError::ArithmeticOverflow)?
            .checked_div(k).ok_or(ProgramError::ArithmeticOverflow)?;

        let amount_a: u64 = (self.pool_ata_a.amount as u128)
        .checked_sub((self.pool_ata_a.amount as u128)
            .checked_mul(ratio).ok_or(ProgramError::ArithmeticOverflow)?
            .checked_div(1000000u128).ok_or(ProgramError::ArithmeticOverflow)?
        ).ok_or(ProgramError::ArithmeticOverflow)? as u64;

        let amount_b: u64 = (self.pool_ata_b.amount as u128)
        .checked_sub((self.pool_ata_b.amount as u128)
            .checked_mul(ratio).ok_or(ProgramError::ArithmeticOverflow)?
            .checked_div(1000000u128).ok_or(ProgramError::ArithmeticOverflow)?
        ).ok_or(ProgramError::ArithmeticOverflow)? as u64;

        // Check slippage A
        require_gte!(amount_a, min_token_a);

        // Check slippage B
        require_gte!(amount_b, min_token_b);

        let binding = self.pool.fee.to_le_bytes();

        let signer_seeds: [&[&[u8]];1] = [&[&b"pool"[..], self.mint_a.to_account_info().key.as_ref(), self.mint_b.to_account_info().key.as_ref(), binding.as_ref(), &[self.pool.bump]]];

        // Withdraw Token A Amount
        let accounts = Transfer {
            from: self.pool_ata_a.to_account_info(),
            to: self.signer_ata_a.to_account_info(),
            authority: self.pool.to_account_info(),
        };

        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(), 
            accounts,
            &signer_seeds
        );
        
        transfer(ctx, amount_a)?;

        // Deposit Token B Amount
        let accounts = Transfer {
            from: self.pool_ata_b.to_account_info(),
            to: self.signer_ata_b.to_account_info(),
            authority: self.pool.to_account_info(),
        };

        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(), 
            accounts,
            &signer_seeds
        );
        
        transfer(ctx, amount_b)?;

        // Burn LP Token
        let accounts = Burn {
            mint: self.mint_lp.to_account_info(),
            from: self.signer_ata_lp.to_account_info(),
            authority: self.signer.to_account_info(),
        };

        let ctx = CpiContext::new(
            self.token_program.to_account_info(), 
            accounts
        );

        burn(ctx, amount)
    }
}
