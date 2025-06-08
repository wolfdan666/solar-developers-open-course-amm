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
            b2 = k / a2
        */
        let k = (self.pool_ata_a.amount as u128)
            .checked_mul(self.pool_ata_b.amount.into()).ok_or(ProgramError::ArithmeticOverflow)?;

        // 我理解了，这里 is_a 确实是 signer 想要 a , 付出 b
        // amount_in 是 signer 想要付出的 b 数量基础数量, 
        // 后面会乘以 10000 + fee 再除以 10000 得到实际付出的 b 数量
        // 所以 max_amount_in 也是 pool 的进入 b 的最大数量，也就是用户付出的最大滑点。
        // 下面的from和to的cpi确实证明上面的signer_in 和 pool_in 是对应的，
        // 但是看起来很难看懂，所以还是改一下试试
        let (signer_in, signer_out, pool_in, pool_out, amount_in) = if is_a {
            // 用户想要获得 amount 个 TokenA，需要付出 TokenB
            let a2 = self.pool_ata_a.amount.checked_sub(amount).ok_or(ProgramError::ArithmeticOverflow)?;
            
            // 🔧 修复：精确计算，避免过早的向上取整
            // 直接计算精确的 amount_in，而不是先计算 b2
            // amount_in = (k / a2) - current_b = k / a2 - pool_b
            // 为了避免精度损失，我们计算: amount_in = (k - a2 * pool_b) / a2
            let numerator = k.checked_sub((a2 as u128).checked_mul(self.pool_ata_b.amount as u128)
                .ok_or(ProgramError::ArithmeticOverflow)?)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            
            let amount_in_exact = numerator.checked_div(a2 as u128)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            
            (
                self.signer_ata_a.to_account_info(),
                self.signer_ata_b.to_account_info(),
                self.pool_ata_b.to_account_info(),
                self.pool_ata_a.to_account_info(),
                // 按理来说，k=ab是池子的恒定值，所以不应该是signer的k，所以池子是b2，signer才应该账户出账b2-pool.b.amount
                amount_in_exact
            )
        } else {
            // 用户想要获得 amount 个 TokenB，需要付出 TokenA
            let b2 = self.pool_ata_b.amount.checked_sub(amount).ok_or(ProgramError::ArithmeticOverflow)?;
            
            // 🔧 修复：精确计算，避免过早的向上取整
            // amount_in = (k / b2) - current_a = k / b2 - pool_a
            // 为了避免精度损失，我们计算: amount_in = (k - b2 * pool_a) / b2
            let numerator = k.checked_sub((b2 as u128).checked_mul(self.pool_ata_a.amount as u128)
                .ok_or(ProgramError::ArithmeticOverflow)?)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            
            let amount_in_exact = numerator.checked_div(b2 as u128)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            
            (
                self.signer_ata_b.to_account_info(),
                self.signer_ata_a.to_account_info(),
                self.pool_ata_a.to_account_info(),
                self.pool_ata_b.to_account_info(),
                amount_in_exact
            )
        };

        // 🔧 修复：只在最终手续费计算时向上取整，确保手续费被正确收取
        // amount_in_with_fees = ceiling(amount_in * (10000 + fee) / 10000)
        let fee_multiplier = 10_000u128 + self.pool.fee as u128;
        let amount_with_fees_exact = amount_in
            .checked_mul(fee_multiplier)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        
        // 向上取整确保手续费不会因为整数除法而丢失
        let amount_in_with_fees: u64 = amount_with_fees_exact
            .checked_add(10_000u128 - 1)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .checked_div(10_000u128)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .try_into().map_err(|_| ProgramError::ArithmeticOverflow)?;

        // Check slippage
        require_gte!(max_amount_in, amount_in_with_fees);

        // is_a: signer out B to pool B
        let accounts = Transfer {
            from: signer_out,
            to: pool_in,
            authority: self.signer.to_account_info()
        };

        let ctx = CpiContext::new(
            self.token_program.to_account_info(), 
            accounts
        );
        
        transfer(ctx, amount_in_with_fees)?;

        // is_a: pool out A to signer A
        let accounts = Transfer {
            from: pool_out,
            to: signer_in,
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