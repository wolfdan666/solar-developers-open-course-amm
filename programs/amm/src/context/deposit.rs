use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token::{mint_to, transfer, Mint, MintTo, Token, TokenAccount, Transfer}};

use crate::state::Pool;

#[derive(Accounts)]
pub struct Deposit<'info> {
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

impl<'info> Deposit<'info> {
    pub fn deposit(&mut self, amount: u64, max_token_a: u64, max_token_b: u64) -> Result<()> {
        let (amount_a, amount_b, amount_lp) = if self.pool_ata_a.amount == 0 && self.pool_ata_b.amount == 0 {
            let k = max_token_a.checked_mul(max_token_b).ok_or(ProgramError::ArithmeticOverflow)?;
            (max_token_a, max_token_b, k)
        } else {
            let k = (self.pool_ata_a.amount as u128).checked_mul(self.pool_ata_b.amount.into()).ok_or(ProgramError::ArithmeticOverflow)?;

            let k2 = k.checked_add(amount as u128).ok_or(ProgramError::ArithmeticOverflow)?;
            let ratio = k2.checked_mul(1000000).ok_or(ProgramError::ArithmeticOverflow)?
                .checked_div(k).ok_or(ProgramError::ArithmeticOverflow)?;

            let amount_a: u64 = ratio.checked_mul(self.pool_ata_a.amount.into()).ok_or(ProgramError::ArithmeticOverflow)?.checked_div(1000000).ok_or(ProgramError::ArithmeticOverflow)?.checked_sub(self.pool_ata_a.amount.into())
            .ok_or(ProgramError::ArithmeticOverflow)?
            .try_into().map_err(|_| ProgramError::ArithmeticOverflow)?;

            let amount_b: u64 = ratio.checked_mul(self.pool_ata_b.amount.into()).ok_or(ProgramError::ArithmeticOverflow)?.checked_div(1000000).ok_or(ProgramError::ArithmeticOverflow)?.checked_sub(self.pool_ata_b.amount.into())
            .ok_or(ProgramError::ArithmeticOverflow)?
            .try_into().map_err(|_| ProgramError::ArithmeticOverflow)?;

            // Check slippage A
            require_gte!(max_token_a, amount_a);

            // Check slippage B
            require_gte!(max_token_b, amount_b);
            (amount_a, amount_b, amount)
        };

        // Deposit Token A Amount
        let accounts = Transfer {
            from: self.signer_ata_a.to_account_info(),
            to: self.pool_ata_a.to_account_info(),
            authority: self.signer.to_account_info(),
        };

        let ctx = CpiContext::new(
            self.token_program.to_account_info(), 
            accounts
        );
        
        transfer(ctx, amount_a)?;

        // Deposit Token B Amount
        let accounts = Transfer {
            from: self.signer_ata_b.to_account_info(),
            to: self.pool_ata_b.to_account_info(),
            authority: self.signer.to_account_info(),
        };

        let ctx = CpiContext::new(
            self.token_program.to_account_info(), 
            accounts
        );
        
        transfer(ctx, amount_b)?;

        // Mint LP Token
        let accounts = MintTo {
            mint: self.mint_lp.to_account_info(),
            to: self.signer_ata_lp.to_account_info(),
            authority: self.pool.to_account_info(),
        };

        let binding = self.pool.fee.to_le_bytes();

        let signer_seeds: [&[&[u8]];1] = [&[&b"pool"[..], self.mint_a.to_account_info().key.as_ref(), self.mint_b.to_account_info().key.as_ref(), binding.as_ref(), &[self.pool.bump]]];

        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(), 
            accounts, 
            &signer_seeds
        );

        mint_to(ctx, amount_lp)
    }
}
