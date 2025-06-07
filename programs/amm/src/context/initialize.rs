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
        // 这里的 set_inner 是将数据写入到已经初始化的 Pool 账户中
        // bump 和 lp_bump 不是传入给账户初始化的参数，而是：
        // 1. 在账户验证阶段，Anchor 已经为 pool 和 mint_lp 这两个 PDA 计算了 canonical bump
        // 2. 这些 bump 值存储在 ctx.bumps 中
        // 3. 现在我们将这些预计算的 bump 值存储到 Pool 数据结构中，作为状态的一部分
        // 4. 存储 bump 的目的是为了后续操作（如签名）时能够重新生成正确的 PDA 地址
        self.pool.set_inner(Pool {
            mint_a: self.mint_a.key(),
            mint_b: self.mint_b.key(),   
            fee,
            bump,      // pool PDA 的 canonical bump，用于后续重新生成 pool 地址
            lp_bump,   // LP mint PDA 的 canonical bump，用于后续 LP token 相关操作
        });
        Ok(())
    }
}
