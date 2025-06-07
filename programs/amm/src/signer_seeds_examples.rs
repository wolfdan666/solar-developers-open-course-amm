use anchor_lang::prelude::*;

/// ========================================
/// Signer Seeds 三重引用详解
/// ========================================

// 示例 1: 单个 PDA 签名（我们的 AMM 场景）
pub fn single_pda_example() {
    // 场景：只需要 pool PDA 签名
    
    let mint_a_key = Pubkey::default();
    let mint_b_key = Pubkey::default();
    let fee_bytes = [0u8; 2];
    let bump = 255u8;
    
    // 类型分解：
    // - `&[u8]`: 单个种子（字节切片的引用）
    let seed1: &[u8] = b"pool";
    let seed2: &[u8] = mint_a_key.as_ref();
    let seed3: &[u8] = mint_b_key.as_ref();
    let seed4: &[u8] = fee_bytes.as_ref();
    let seed5: &[u8] = &[bump];
    
    // - `&[&[u8]]`: 种子组（生成一个 PDA 的所有种子）
    let seeds_for_one_pda: &[&[u8]] = &[seed1, seed2, seed3, seed4, seed5];
    
    // - `[&[&[u8]]; 1]`: 种子组数组（可以包含多个 PDA 的种子组）
    let signer_seeds: [&[&[u8]]; 1] = [seeds_for_one_pda];
    
    // 最终传递给 CpiContext::new_with_signer 的是 &[&[&[u8]]]
    // let ctx = CpiContext::new_with_signer(program, accounts, &signer_seeds);
}

// 示例 2: 多个 PDA 签名（复杂场景）
pub fn multiple_pda_example() {
    // 场景：需要 2 个不同的 PDA 同时签名
    
    let user_key = Pubkey::default();
    let pool_key = Pubkey::default();
    let bump1 = 254u8;
    let bump2 = 253u8;
    
    // 第一个 PDA 的种子组
    let pda1_seeds: &[&[u8]] = &[
        b"user_vault",
        user_key.as_ref(),
        &[bump1]
    ];
    
    // 第二个 PDA 的种子组  
    let pda2_seeds: &[&[u8]] = &[
        b"pool_vault",
        pool_key.as_ref(),
        &[bump2]
    ];
    
    // 多个 PDA 的种子组数组
    let signer_seeds: [&[&[u8]]; 2] = [pda1_seeds, pda2_seeds];
    
    // 传递给 CPI: &[&[&[u8]]]
    // let ctx = CpiContext::new_with_signer(program, accounts, &signer_seeds);
}

// 示例 3: 类型推导简化写法（常见用法）
pub fn simplified_syntax_example() {
    let mint_key = Pubkey::default();
    let bump = 255u8;
    
    // 直接内联写法，让编译器推导类型
    let signer_seeds = [&[
        b"authority".as_ref(),
        mint_key.as_ref(),
        &[bump]
    ]];
    
    // 等价于：
    // let signer_seeds: [&[&[u8]]; 1] = [&[
    //     b"authority".as_ref(),
    //     mint_key.as_ref(), 
    //     &[bump]
    // ]];
}

/// ========================================
/// 为什么要这样设计？
/// ========================================
//
// 1. **灵活性**：支持一个指令中多个 PDA 签名
//    - DeFi 协议可能需要多个账户权限
//    - 复杂交易可能涉及多个程序控制的资源
//
// 2. **类型安全**：编译时确保正确的数据结构
//    - 防止种子类型错误
//    - 确保引用生命周期正确
//
// 3. **性能优化**：零拷贝引用传递
//    - 不需要克隆大量数据
//    - 高效的内存使用
//
// 4. **API 一致性**：统一的签名接口
//    - 无论单个还是多个 PDA，API 保持一致
//    - 简化框架设计和使用
//
// 这种设计虽然看起来复杂，但提供了极大的灵活性和类型安全，是 Solana 生态系统中优雅的解决方案！
/// ========================================
/// 实际使用技巧
/// ========================================

// 技巧 1: 使用变量简化复杂的种子构造
pub fn clean_seeds_construction() {
    let program_id = Pubkey::default();
    let user = Pubkey::default();
    let mint = Pubkey::default();
    let bump = 255u8;
    
    // 分步构造，提高可读性
    let seed_literal = b"user_account";
    let seed_program = program_id.as_ref();
    let seed_user = user.as_ref();
    let seed_mint = mint.as_ref();
    let seed_bump = [bump];
    
    let signer_seeds = [&[
        seed_literal.as_ref(),
        seed_program,
        seed_user,
        seed_mint,
        seed_bump.as_ref(),
    ]];
}

// 技巧 2: 在实际使用中，直接内联构造更简洁
// 避免不必要的函数抽象和生命周期复杂性 