use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer, MintTo, Burn, Token, TokenAccount, Mint};

pub mod state;
pub mod context;
pub mod cpi_examples;  // CPI 调用示例模块
pub mod signer_seeds_examples;  // Signer Seeds 三重引用详解模块

pub use context::*;

declare_id!("2wmWHXHy6F3Yz2CaaWNp7Bfgz4c4kpPXuHiDyVajZARu");

// ========================================
// CPI (Cross Program Invocation) 调用示例
// ========================================
//
// CPI 是 Solana 中一个程序调用另一个程序的机制，主要用途：
// 1. **Token 操作**：调用 SPL Token 程序进行转账、铸造、销毁等
// 2. **Associated Token Account**：创建关联代币账户
// 3. **程序组合**：将多个程序功能组合成复杂的 DeFi 协议
// 4. **权限委托**：通过 PDA 签名将权限传递给其他程序
//
// CPI 的两种类型：
// - **普通 CPI**：使用现有签名者的权限
// - **PDA CPI**：使用 PDA 作为签名者（程序签名）
//
// 在 AMM 中的 CPI 使用场景：
// - 转移用户代币到池子
// - 从池子转移代币给用户  
// - 铸造 LP 代币
// - 销毁 LP 代币

#[program]
pub mod amm {
    use super::*;

    /// 初始化 AMM 流动性池
    /// 
    /// bump 在账户验证阶段自动生成，然后在 initialize 函数中显性获取并存储。
    /// 
    /// 执行流程：
    /// 1. Anchor 在验证 Context<Initialize> 时，为带有 #[account(init, ..., bump)] 的 PDA 自动计算 canonical bump
    /// 2. 这些 bump 值存储在 ctx.bumps 中（ctx.bumps.pool 和 ctx.bumps.mint_lp）
    /// 3. 在 initialize 函数中显性获取这些 bump 值
    /// 4. 通过 set_inner 将 bump 值存储到 Pool 账户的数据中，作为状态的一部分
    /// 5. 后续操作中可以使用存储的 bump 值重新生成正确的 PDA 地址进行签名
    /// 
    /// 显性获取 bumps 的好处：
    /// 1. **安全性优势**：避免在运行时重新计算 PDA，减少潜在的安全风险
    /// 2. **性能优化**：预先计算好的 bump 值避免了昂贵的重复计算过程
    /// 3. **确定性保证**：确保使用正确的 canonical bump，防止恶意攻击者提供错误的 bump
    /// 4. **代码透明性**：明确显示哪些 PDA 被使用，提高代码可读性和可审计性
    /// 5. **Gas 效率**：减少指令执行时间，降低交易成本
    pub fn initialize(ctx: Context<Initialize>, fee: u16) -> Result<()> {
        // 显性获取并传递 bumps：
        // - ctx.bumps.pool: 从 Context 中获取 pool PDA 的 canonical bump
        // - ctx.bumps.mint_lp: 从 Context 中获取 LP token mint PDA 的 canonical bump
        // 这些 bump 值由 Anchor 框架在账户验证阶段自动计算并存储在 ctx.bumps 中
        // 然后传入 initialize 实现函数，最终存储到 Pool 账户数据中
        ctx.accounts.initialize(fee, ctx.bumps.pool, ctx.bumps.mint_lp)
    }

    /// 向流动性池存入代币，获得 LP 代币
    /// amount: 期望的 LP 代币数量
    /// max_token_a/max_token_b: 愿意支付的最大代币数量（滑点保护）
    pub fn deposit(ctx: Context<Deposit>, amount: u64, max_token_a: u64, max_token_b: u64) -> Result<()> {
        ctx.accounts.deposit(amount, max_token_a, max_token_b)
    }

    /// 从流动性池提取代币，销毁 LP 代币
    /// amount: 要销毁的 LP 代币数量
    /// min_token_a/min_token_b: 期望获得的最小代币数量（滑点保护）
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64, min_token_a: u64, min_token_b: u64) -> Result<()> {
        ctx.accounts.withdraw(amount, min_token_a, min_token_b)
    }

    /// 在流动性池中交换代币
    /// amount: 期望获得的输出代币数量
    /// max_amount_in: 愿意支付的最大输入代币数量（滑点保护）
    /// is_a: true 表示用 token_a 换 token_b，false 表示用 token_b 换 token_a
    pub fn swap(ctx: Context<Swap>, amount: u64, max_amount_in: u64, is_a: bool) -> Result<()> {
        ctx.accounts.swap(amount, max_amount_in, is_a)
    }
}