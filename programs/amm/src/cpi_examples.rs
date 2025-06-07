use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer, MintTo, Burn, Token, TokenAccount, Mint};

/// ========================================
/// CPI 调用示例大全
/// ========================================

// 1. 普通 CPI：用户签名的代币转账
pub fn transfer_tokens_user_signed<'info>(
    from: &Account<'info, TokenAccount>,
    to: &Account<'info, TokenAccount>,
    authority: &Signer<'info>,
    token_program: &Program<'info, Token>,
    amount: u64,
) -> Result<()> {
    // 创建 CPI Context
    let cpi_accounts = Transfer {
        from: from.to_account_info(),
        to: to.to_account_info(),
        authority: authority.to_account_info(),
    };
    
    let cpi_program = token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    
    // 执行 CPI 调用
    token::transfer(cpi_ctx, amount)?;
    
    Ok(())
}

// 2. PDA CPI：程序代表池子签名的代币转账
pub fn transfer_tokens_pda_signed<'info>(
    from: &Account<'info, TokenAccount>,
    to: &Account<'info, TokenAccount>,
    pool_authority: &AccountInfo<'info>,  // PDA 账户
    token_program: &Program<'info, Token>,
    amount: u64,
    pool_bump: u8,
    pool_seeds: &[&[u8]],  // 用于生成 PDA 的种子
) -> Result<()> {
    // 创建 CPI Context
    let cpi_accounts = Transfer {
        from: from.to_account_info(),
        to: to.to_account_info(),
        authority: pool_authority.clone(),
    };
    
    let cpi_program = token_program.to_account_info();
    
    // 构造签名种子（包含 bump）
    let mut signer_seeds = pool_seeds.to_vec();
    let bump_slice = &[pool_bump];
    signer_seeds.push(bump_slice);
    let signer_seeds_slice: Vec<&[u8]> = signer_seeds.iter().map(|s| s.as_ref()).collect();
    
    // 使用 PDA 签名创建 CPI Context
    let signer_seeds_array = [&signer_seeds_slice[..]];
    let cpi_ctx = CpiContext::new_with_signer(
        cpi_program,
        cpi_accounts,
        &signer_seeds_array
    );
    
    // 执行 CPI 调用
    token::transfer(cpi_ctx, amount)?;
    
    Ok(())
}

// 3. 铸造代币的 CPI
pub fn mint_tokens<'info>(
    mint: &Account<'info, Mint>,
    to: &Account<'info, TokenAccount>,
    mint_authority: &AccountInfo<'info>,  // 通常是 PDA
    token_program: &Program<'info, Token>,
    amount: u64,
    authority_bump: u8,
    authority_seeds: &[&[u8]],
) -> Result<()> {
    let cpi_accounts = MintTo {
        mint: mint.to_account_info(),
        to: to.to_account_info(),
        authority: mint_authority.clone(),
    };
    
    let cpi_program = token_program.to_account_info();
    
    // 构造签名种子
    let mut signer_seeds = authority_seeds.to_vec();
    let bump_slice = &[authority_bump];
    signer_seeds.push(bump_slice);
    let signer_seeds_slice: Vec<&[u8]> = signer_seeds.iter().map(|s| s.as_ref()).collect();
    
    let signer_seeds_array = [&signer_seeds_slice[..]];
    let cpi_ctx = CpiContext::new_with_signer(
        cpi_program,
        cpi_accounts,
        &signer_seeds_array
    );
    
    token::mint_to(cpi_ctx, amount)?;
    
    Ok(())
}

// 4. 销毁代币的 CPI
pub fn burn_tokens<'info>(
    mint: &Account<'info, Mint>,
    from: &Account<'info, TokenAccount>,
    authority: &Signer<'info>,  // 代币持有者
    token_program: &Program<'info, Token>,
    amount: u64,
) -> Result<()> {
    let cpi_accounts = Burn {
        mint: mint.to_account_info(),
        from: from.to_account_info(),
        authority: authority.to_account_info(),
    };
    
    let cpi_program = token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    
    token::burn(cpi_ctx, amount)?;
    
    Ok(())
}

// ========================================
// CPI 最佳实践和注意事项
// ========================================

// 1. **权限检查**：
//    - 确保调用者有权限执行操作
//    - 验证 PDA 的种子和 bump 正确性
//
// 2. **错误处理**：
//    - CPI 调用可能失败，需要适当的错误处理
//    - 使用 Result<()> 返回类型
//
// 3. **Gas 优化**：
//    - CPI 调用有额外的计算成本
//    - 合理组织调用顺序和次数
//
// 4. **安全考虑**：
//    - 验证被调用程序的 program_id
//    - 检查账户所有权和状态
//    - 防止重入攻击
//
// 5. **调试技巧**：
//    - 使用 msg! 宏记录 CPI 调用前后的状态
//    - 检查账户余额变化
//    - 验证 CPI 返回值 