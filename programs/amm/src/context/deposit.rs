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

            let amount_a: u64 = ratio.checked_mul(self.pool_ata_a.amount.into()).ok_or(ProgramError::ArithmeticOverflow)?
                                     .checked_div(1000000).ok_or(ProgramError::ArithmeticOverflow)?
                                     .checked_sub(self.pool_ata_a.amount.into()).ok_or(ProgramError::ArithmeticOverflow)?
                                     .try_into().map_err(|_| ProgramError::ArithmeticOverflow)?;

            let amount_b: u64 = ratio.checked_mul(self.pool_ata_b.amount.into()).ok_or(ProgramError::ArithmeticOverflow)?
                                     .checked_div(1000000).ok_or(ProgramError::ArithmeticOverflow)?
                                     .checked_sub(self.pool_ata_b.amount.into()).ok_or(ProgramError::ArithmeticOverflow)?
                                     .try_into().map_err(|_| ProgramError::ArithmeticOverflow)?;

            // Check slippage A
            require_gte!(max_token_a, amount_a);

            // Check slippage B
            require_gte!(max_token_b, amount_b);
            (amount_a, amount_b, amount)
        };

        // ==========================================
        // CPI 调用 1: 转移 Token A 到池子 (用户签名)
        // ==========================================
        // 这是一个普通的 CPI 调用，用户签名授权转移自己的代币
        let accounts = Transfer {
            from: self.signer_ata_a.to_account_info(),  // 源账户：用户的 Token A 账户
            to: self.pool_ata_a.to_account_info(),      // 目标账户：池子的 Token A 账户
            authority: self.signer.to_account_info(),    // 权限：用户签名者
        };

        let ctx = CpiContext::new(
            self.token_program.to_account_info(),   // 被调用程序：SPL Token 程序
            accounts
        );
        
        // 调用 SPL Token 程序的 transfer 指令
        transfer(ctx, amount_a)?;

        // ==========================================
        // CPI 调用 2: 转移 Token B 到池子 (用户签名)
        // ==========================================
        // 同样是普通 CPI 调用，转移用户的 Token B
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

        // ==========================================
        // CPI 调用 3: 铸造 LP 代币 (PDA 签名)
        // ==========================================
        // 这是一个 PDA CPI 调用，池子作为 LP token 的 mint authority
        let accounts = MintTo {
            mint: self.mint_lp.to_account_info(),       // LP token mint 账户
            to: self.signer_ata_lp.to_account_info(),   // 目标：用户的 LP token 账户
            authority: self.pool.to_account_info(),     // 权限：池子 PDA（mint authority）
        };

        // ==========================================
        // Pool Fee 详解
        // ==========================================
        // 
        // pool.fee 有两个重要作用：
        // 
        // 1. **PDA 种子区分器**：
        //    - fee 是生成 pool PDA 的种子之一
        //    - 同一对代币 (mint_a, mint_b) 可以创建多个不同手续费率的池子
        //    - 例如：USDC/USDT 可以有 0.01%, 0.3%, 1% 等不同费率的池子
        //    - 不同 fee 生成不同的 PDA 地址，实现池子隔离
        //
        // 2. **交易手续费率**：
        //    - 在 swap 操作中，fee 用于计算实际手续费
        //    - 公式：amount_in_with_fees = amount_in * (10000 + fee) / 10000
        //    - fee 以基点为单位：100 = 1%, 30 = 0.3%, 1 = 0.01%
        //    - deposit/withdraw 操作不收手续费，只有 swap 收取
        // 总结：pool.fee 不是 deposit 时的手续费，而是用于区分不同费率池子的标识符，实际的手续费只在 swap 交易时收取！
        
        let binding = self.pool.fee.to_le_bytes();

        // ==========================================
        // 三重引用的 signer_seeds 类型解析
        // ==========================================
        // 
        // 类型签名：[&[&[u8]]; 1]
        // 
        // 层次结构解析：
        // 1. 最内层 `&[u8]`     -> 单个种子的字节切片引用
        // 2. 中间层 `&[&[u8]]`  -> 种子组的引用（生成一个 PDA 所需的所有种子）
        // 3. 最外层 `[&[&[u8]]; 1]` -> 种子组数组（支持多个 PDA 同时签名）
        //
        // 为什么需要三重引用？
        // - Solana 指令可能需要多个 PDA 同时签名
        // - 每个 PDA 需要一组种子来生成
        // - 每组种子包含多个字节切片
        // - 因此形成了三层嵌套的引用结构
        //
        // 实际使用中：
        // - 我们只需要一个 PDA (pool) 签名，所以数组长度为 1
        // - 这个 PDA 需要 5 个种子：["pool", mint_a, mint_b, fee, bump]
        // - 每个种子都是 &[u8] 类型
        
        let signer_seeds: [&[&[u8]]; 1] = [&[
            &b"pool"[..],                                    // 种子 1: "pool" 字面量
            self.mint_a.to_account_info().key.as_ref(),     // 种子 2: mint_a 公钥
            self.mint_b.to_account_info().key.as_ref(),     // 种子 3: mint_b 公钥  
            binding.as_ref(),                               // 种子 4: fee 参数
            &[self.pool.bump]                               // 种子 5: canonical bump
        ]];

        // 使用 PDA 签名创建 CPI Context
        // new_with_signer 允许程序代表 PDA 进行签名
        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),   // SPL Token 程序
            accounts, 
            &signer_seeds                           // PDA 签名种子：&[&[&[u8]]]
        );

        // 调用 SPL Token 程序的 mint_to 指令，铸造 LP 代币给用户
        mint_to(ctx, amount_lp)
    }
}
