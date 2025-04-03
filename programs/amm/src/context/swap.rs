use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token::{transfer, Mint, Token, TokenAccount, Transfer}};

use crate::state::Pool;

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    signer: Signer<'info>,
    mint_a: Account<'info, Mint>,
    mint_b: Account<'info, Mint>,
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

impl<'info> Swap<'info> {
    pub fn swap(&mut self, amount: u64, max_amount_in: u64, is_a: bool) -> Result<()> {
        /*
            k = ab
            a2 = a - amount 
            b2 = k2 / a2
        */
        let k = (self.pool_ata_a.amount as u128)
            .checked_mul(self.pool_ata_b.amount.into()).ok_or(ProgramError::ArithmeticOverflow)?;

        let (signer_in, signer_out, pool_in, pool_out, amount_in) = if is_a {
            let a2 = self.pool_ata_a.amount.checked_sub(amount).ok_or(ProgramError::ArithmeticOverflow)?;
            let b2: u64 = k.checked_div(a2.into()).ok_or(ProgramError::ArithmeticOverflow)?.try_into().map_err(|_| ProgramError::ArithmeticOverflow)?;    
            (
                self.signer_ata_b.to_account_info(),
                self.signer_ata_a.to_account_info(),
                self.pool_ata_b.to_account_info(),
                self.pool_ata_a.to_account_info(),
                b2 - self.pool_ata_b.amount
            )
        } else {
            let b2 = self.pool_ata_b.amount.checked_sub(amount).ok_or(ProgramError::ArithmeticOverflow)?;
            let a2: u64 = k.checked_div(b2.into()).ok_or(ProgramError::ArithmeticOverflow)?.try_into().map_err(|_| ProgramError::ArithmeticOverflow)?;
            (
                self.signer_ata_b.to_account_info(),
                self.signer_ata_a.to_account_info(),
                self.pool_ata_b.to_account_info(),
                self.pool_ata_a.to_account_info(),
                a2 - self.pool_ata_a.amount
            )
        };

        let amount_in_with_fees: u64 = (amount_in as u128)
            .checked_mul(10_000 + self.pool.fee as u128)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .checked_div(10_000).ok_or(ProgramError::ArithmeticOverflow)?
            .try_into().map_err(|_| ProgramError::ArithmeticOverflow)?;

        // Check slippage
        require_gte!(max_amount_in, amount_in_with_fees);

        // Deposit Token In Amount
        let accounts = Transfer {
            from: signer_in,
            to: pool_in,
            authority: self.signer.to_account_info()
        };

        let ctx = CpiContext::new(
            self.token_program.to_account_info(), 
            accounts
        );
        
        transfer(ctx, amount_in_with_fees)?;

        // Deposit Token B Amount
        let accounts = Transfer {
            from: pool_out,
            to: signer_out,
            authority: self.pool.to_account_info(),
        };

        let binding = self.pool.fee.to_le_bytes();

        let signer_seeds: [&[&[u8]];1] = [&[&b"pool"[..], self.mint_a.to_account_info().key.as_ref(), self.mint_b.to_account_info().key.as_ref(), binding.as_ref(), &[self.pool.bump]]];

        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(), 
            accounts,
            &signer_seeds
        );
        
        transfer(ctx, amount)
    }
}