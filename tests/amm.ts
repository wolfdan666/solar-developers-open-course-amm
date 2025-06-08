import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Amm } from "../target/types/amm";
import { Keypair, LAMPORTS_PER_SOL, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import { BN, min } from "bn.js";
import { ASSOCIATED_PROGRAM_ID, TOKEN_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
import { createAssociatedTokenAccountIdempotentInstruction, createInitializeMint2Instruction, createMintToInstruction, getAssociatedTokenAddressSync, getMinimumBalanceForRentExemptMint, MINT_SIZE } from "@solana/spl-token";

describe("amm", () => {
  // ========================================
  // Anchor 框架初始化和连接设置
  // ========================================
  
  // 1. 设置 Anchor Provider (提供者)
  // AnchorProvider.env() 从环境变量中读取配置：
  // - ANCHOR_PROVIDER_URL: RPC 端点 URL (例如: http://127.0.0.1:8899)
  // - ANCHOR_WALLET: 钱包路径 (例如: ~/.config/solana/id.json)
  // Provider 负责管理与 Solana 区块链的连接和钱包签名
  anchor.setProvider(anchor.AnchorProvider.env());

  // 2. 获取已设置的 provider 实例
  // provider 包含了连接信息、钱包信息等，是与区块链交互的桥梁
  const provider = anchor.getProvider();

  // 3. 从 provider 中提取 Solana 连接对象
  // connection 是与 Solana RPC 节点通信的核心对象，用于：
  // - 发送交易 (sendTransaction)
  // - 查询账户状态 (getAccountInfo)
  // - 获取区块信息 (getLatestBlockhash)
  const connection = provider.connection;

  // 4. 获取 AMM 程序实例
  // anchor.workspace.amm 自动从 target/idl/amm.json 中加载程序接口
  // Program<Amm> 是 TypeScript 类型标注，确保类型安全
  // 这个对象包含了所有程序方法：initialize, deposit, withdraw, swap
  const program = anchor.workspace.amm as Program<Amm>;

  // ========================================
  // Promise 和异步编程详解
  // ========================================
  
  /**
   * 交易确认函数 - 异步函数和 Promise 语法示例
   * 
   * @param signature - 交易签名字符串
   * @returns Promise<string> - 返回一个 Promise，解析为字符串
   * 
   * Promise 语法解释：
   * 1. async 关键字：声明这是一个异步函数，自动返回 Promise
   * 2. await 关键字：等待 Promise 完成，暂停函数执行直到操作完成
   * 3. Promise<string>：TypeScript 类型标注，表示返回 Promise，解析值类型为 string
   */
  const confirm = async (signature: string): Promise<string> => {
    // await connection.getLatestBlockhash()
    // ↓ Promise 链式调用等价写法：
    // connection.getLatestBlockhash().then(block => { ... })
    
    // 步骤1: 获取最新区块哈希和区块高度
    // 这是异步操作，需要向 RPC 节点发送请求
    // await 会暂停执行，等待网络请求完成
    const block = await connection.getLatestBlockhash();
    
    // 步骤2: 确认交易
    // 使用展开语法 (...block) 将 block 对象的所有属性展开
    // 等价于: { signature: signature, blockhash: block.blockhash, lastValidBlockHeight: block.lastValidBlockHeight }
    await connection.confirmTransaction({
      signature,           // 交易签名
      ...block,           // 展开操作符，包含 blockhash 和 lastValidBlockHeight
    });
    
    // 步骤3: 返回原始签名
    // 在 async 函数中，return 的值会自动包装成 Promise.resolve(value)
    return signature;
  };

  /**
   * 日志输出函数 - 另一个 Promise 示例
   * 
   * 功能: 打印交易签名的区块链浏览器链接
   * 
   * Promise 的三种等价写法对比：
   * 
   * 写法1 (当前使用): async/await 语法
   * const log = async (signature: string): Promise<string> => {
   *   console.log(`链接: ${signature}`);
   *   return signature;
   * }
   * 
   * 写法2: 传统 Promise 构造函数
   * const log = (signature: string): Promise<string> => {
   *   return new Promise((resolve, reject) => {
   *     console.log(`链接: ${signature}`);
   *     resolve(signature);  // 成功时调用
   *   });
   * }
   * 
   * 写法3: Promise.resolve 静态方法
   * const log = (signature: string): Promise<string> => {
   *   console.log(`链接: ${signature}`);
   *   return Promise.resolve(signature);
   * }
   */
  const log = async (signature: string): Promise<string> => {
    console.log(
      `Your transaction signature: https://explorer.solana.com/transaction/${signature}?cluster=custom&customUrl=${connection.rpcEndpoint}`
    );
    return signature;
  };

  /**
   * 查询并打印代币余额函数
   * 
   * 功能：显示池子和用户的TokenA、TokenB余额
   * 这个函数演示了如何查询Solana上的代币账户余额
   */
  const logBalances = async (testName: string): Promise<void> => {
    console.log(`\n========== ${testName} 余额查询 ==========`);
    
    try {
      // 查询池子的TokenA余额
      const poolAtaAInfo = await connection.getTokenAccountBalance(poolAtaA);
      // 查询池子的TokenB余额  
      const poolAtaBInfo = await connection.getTokenAccountBalance(poolAtaB);
      
      // 查询用户的TokenA余额
      const signerAtaAInfo = await connection.getTokenAccountBalance(signerAtaA);
      // 查询用户的TokenB余额
      const signerAtaBInfo = await connection.getTokenAccountBalance(signerAtaB);
      
      // 尝试查询LP代币余额（可能不存在）
      let signerAtaLpInfo;
      try {
        signerAtaLpInfo = await connection.getTokenAccountBalance(signerAtaLp);
      } catch (error) {
        signerAtaLpInfo = { value: { amount: "0", decimals: 6, uiAmount: 0 } };
      }

      console.log("🏊‍♂️ 池子余额:");
      console.log(`  TokenA: ${Number(poolAtaAInfo.value.amount) / 1e6} (${poolAtaAInfo.value.amount} 原始单位)`);
      console.log(`  TokenB: ${Number(poolAtaBInfo.value.amount) / 1e6} (${poolAtaBInfo.value.amount} 原始单位)`);
      
      console.log("👤 用户余额:");
      console.log(`  TokenA: ${Number(signerAtaAInfo.value.amount) / 1e6} (${signerAtaAInfo.value.amount} 原始单位)`);
      console.log(`  TokenB: ${Number(signerAtaBInfo.value.amount) / 1e6} (${signerAtaBInfo.value.amount} 原始单位)`);
      console.log(`  LP Token: ${Number(signerAtaLpInfo.value.amount) / 1e6} (${signerAtaLpInfo.value.amount} 原始单位)`);
      
      // 计算总体统计
      const totalTokenA = Number(poolAtaAInfo.value.amount) + Number(signerAtaAInfo.value.amount);
      const totalTokenB = Number(poolAtaBInfo.value.amount) + Number(signerAtaBInfo.value.amount);
      
      console.log("📊 总计:");
      console.log(`  TokenA总量: ${totalTokenA / 1e6} (${totalTokenA} 原始单位)`);
      console.log(`  TokenB总量: ${totalTokenB / 1e6} (${totalTokenB} 原始单位)`);
      
      // 计算池子的常数乘积 k = a * b
      const poolA = Number(poolAtaAInfo.value.amount);
      const poolB = Number(poolAtaBInfo.value.amount);
      const k = poolA * poolB;
      
      if (poolA > 0 && poolB > 0) {
        console.log("🔢 池子常数乘积:");
        console.log(`  K = TokenA × TokenB = ${poolA} × ${poolB} = ${k}`);
        console.log(`  价格比率: 1 TokenA = ${poolB/poolA} TokenB`);
        console.log(`  价格比率: 1 TokenB = ${poolA/poolB} TokenA`);
      }
      
      console.log("=======================================\n");
      
    } catch (error) {
      console.log(`查询余额失败: ${error}`);
    }
  };

  const fee = new BN(500);
  const signer = Keypair.generate();
  const mintA = Keypair.generate();
  const mintB = Keypair.generate();
  const pool = PublicKey.findProgramAddressSync([
    Buffer.from("pool"),
    mintA.publicKey.toBuffer(),
    mintB.publicKey.toBuffer(),
    fee.toArrayLike(Buffer, "le", 2)
  ],
  program.programId)[0];
  const mintLp = PublicKey.findProgramAddressSync([
    Buffer.from("lp"),
    pool.toBuffer()
  ],
  program.programId)[0];

  const tokenProgram = TOKEN_PROGRAM_ID;

  const poolAtaA = getAssociatedTokenAddressSync(
    mintA.publicKey,
    pool,
    true,
    tokenProgram
  );

  const poolAtaB = getAssociatedTokenAddressSync(
    mintB.publicKey,
    pool,
    true,
    tokenProgram
  );

  const signerAtaA = getAssociatedTokenAddressSync(
    mintA.publicKey,
    signer.publicKey,
    false,
    tokenProgram
  );

  const signerAtaB = getAssociatedTokenAddressSync(
    mintB.publicKey,
    signer.publicKey,
    false,
    tokenProgram
  );

  const signerAtaLp = getAssociatedTokenAddressSync(
    mintLp,
    signer.publicKey,
    false,
    tokenProgram
  );

  const accounts = {
    signer: signer.publicKey,
    mintA: mintA.publicKey,
    mintB: mintB.publicKey,
    pool,
    mintLp,
    signerAtaA,
    signerAtaB,
    signerAtaLp,
    poolAtaA,
    poolAtaB,
    systemProgram: SystemProgram.programId,
    tokenProgram,
    associatedTokenProgram: ASSOCIATED_PROGRAM_ID
  }

  it("Airdrop and create mints", async () => {
    // ========================================
    // Promise 错误处理示例
    // ========================================
    
    // await 语法会自动处理 Promise，如果失败会抛出错误
    // 等价于: getMinimumBalanceForRentExemptMint(connection).then(result => result).catch(error => throw error)
    let lamports = await getMinimumBalanceForRentExemptMint(connection);
    
    let tx = new Transaction();
    tx.instructions = [
      // 转账 SOL 给测试签名者
      SystemProgram.transfer({
        fromPubkey: provider.publicKey,
        toPubkey: signer.publicKey,
        lamports: 10 * LAMPORTS_PER_SOL,
      }),
      // 使用数组 map 方法为每个代币创建账户
      ...[mintA, mintB].map((mint) =>
        SystemProgram.createAccount({
          fromPubkey: provider.publicKey,
          newAccountPubkey: mint.publicKey,
          lamports,
          space: MINT_SIZE,
          programId: tokenProgram,
        })
      ),
      // 初始化代币铸币权限
      createInitializeMint2Instruction(mintA.publicKey, 6, provider.publicKey!, null, tokenProgram),
      createInitializeMint2Instruction(mintB.publicKey, 6, provider.publicKey!, null, tokenProgram),
      // 创建关联代币账户 (ATA)
      createAssociatedTokenAccountIdempotentInstruction(provider.publicKey, signerAtaA, signer.publicKey, mintA.publicKey, tokenProgram),
      createAssociatedTokenAccountIdempotentInstruction(provider.publicKey, signerAtaB, signer.publicKey, mintB.publicKey, tokenProgram),
      // 铸造初始代币到用户账户
      createMintToInstruction(mintA.publicKey, signerAtaA, provider.publicKey!, 1e9, undefined, tokenProgram),
      createMintToInstruction(mintB.publicKey, signerAtaB, provider.publicKey!, 1e9, undefined, tokenProgram),
    ];

    /**
     * Promise 错误处理的三种方式:
     * 
     * 方式1: 当前使用 - 简单链式调用 (错误会向上传播)
     * await provider.sendAndConfirm(tx, [mintA, mintB]).then(log);
     * 
     * 方式2: 使用 .catch() 处理错误
     * await provider.sendAndConfirm(tx, [mintA, mintB])
     *   .then(log)
     *   .catch(error => console.error("交易失败:", error));
     * 
     * 方式3: try/catch 语句
     * try {
     *   const signature = await provider.sendAndConfirm(tx, [mintA, mintB]);
     *   await log(signature);
     * } catch (error) {
     *   console.error("交易失败:", error);
     * }
     */
    await provider.sendAndConfirm(tx, [mintA, mintB]).then(log);
    
    // 查询并显示初始余额
    await logBalances("空投和创建代币后");
  });

  it("Initialize a pool", async () => {
    // ========================================
    // Promise 链式调用示例
    // ========================================
    
    /**
     * 下面展示了三种不同的 Promise 使用方式:
     * 
     * 方式1: 当前使用的链式调用 (.then())
     * program.methods.initialize()...rpc().then(confirm).then(log)
     * 
     * 方式2: async/await 语法 (等价写法)
     * const txSignature = await program.methods.initialize()...rpc();
     * const confirmedTx = await confirm(txSignature);
     * const loggedTx = await log(confirmedTx);
     * 
     * 方式3: 分步执行
     * const promise1 = program.methods.initialize()...rpc();
     * const promise2 = promise1.then(confirm);
     * const promise3 = promise2.then(log);
     * const result = await promise3;
     */
    const tx = await program.methods.initialize(
      fee.toNumber()    // 手续费参数 (500 = 5%)
    )
    .accountsStrict({   // 严格账户验证，必须提供所有必需账户
      ...accounts       // 展开所有预定义账户
    })
    .signers([          // 交易签名者数组
      signer            // 池子创建者
    ])
    .rpc()              // 发送交易到RPC节点，返回 Promise<string> (交易签名)
    .then(confirm)      // Promise链: 确认交易，等价于 .then(signature => confirm(signature))
    .then(log);         // Promise链: 记录日志，等价于 .then(signature => log(signature))
    
    // 执行流程:
    // 1. program.methods.initialize().rpc() 返回 Promise<string>
    // 2. .then(confirm) 接收签名，返回 Promise<string>
    // 3. .then(log) 接收确认后的签名，返回 Promise<string>
    // 4. await 等待整个链式调用完成
    
    // 查询并显示池子初始化后的余额
    await logBalances("初始化池子后");
  });

  it("Deposit", async () => {
    const tx = await program.methods.deposit(
      new BN(625), new BN(25), new BN(25)
    )
    .preInstructions([
      createAssociatedTokenAccountIdempotentInstruction(
        signer.publicKey,
        signerAtaLp,
        signer.publicKey,
        mintLp,
        tokenProgram
      )
    ])
    .accountsStrict({
      ...accounts
    })
    .signers([
      signer
    ])
    .rpc()
    .then(confirm)
    .then(log);
    
    // 查询并显示存入流动性后的余额
    await logBalances("存入流动性后");
  });

  it("Swap", async () => {
    const tx = await program.methods.swap(
      new BN(4), new BN(6), true  // 增加滑点容忍度到6，确保能容纳手续费
    )
    .accountsStrict({
      ...accounts
    })
    .signers([
      signer
    ])
    .rpc()
    .then(confirm)
    .then(log);
    
    // 查询并显示交换后的余额
    await logBalances("交换后");
    
    // 分析手续费情况
    await analyzeSwapFees();
  });

  it("Withdraw", async () => {
    const tx = await program.methods.withdraw(
      new BN(625), new BN(21), new BN(29)
    )
    .accountsStrict({
      ...accounts
    })
    .signers([
      signer
    ])
    .rpc()
    .then(confirm)
    .then(log);
    
    // 查询并显示提取流动性后的余额
    await logBalances("提取流动性后");
  });

  /**
   * 手续费分析函数
   * 详细分析swap交易中的手续费计算
   */
  const analyzeSwapFees = async (): Promise<void> => {
    console.log(`\n🔍 ========== SWAP手续费详细分析 ==========`);
    
    // 获取当前余额
    const poolAtaAInfo = await connection.getTokenAccountBalance(poolAtaA);
    const poolAtaBInfo = await connection.getTokenAccountBalance(poolAtaB);
    
    const poolA = Number(poolAtaAInfo.value.amount);
    const poolB = Number(poolAtaBInfo.value.amount);
    const currentK = poolA * poolB;
    
    console.log("📈 交换分析：");
    console.log(`  用户想要获得: 4 个 TokenA`);
    console.log(`  手续费率: 500 (5.00%)`);
    
    console.log("\n🧮 精确模拟Swap代码计算：");
    const originalA = 25;
    const originalB = 25;
    const originalK = originalA * originalB; // 625
    const wantedA = 4; // 用户想要的TokenA数量
    
    console.log(`  交换前池子状态: TokenA=${originalA}, TokenB=${originalB}, K=${originalK}`);
    console.log(`  交换后池子状态: TokenA=${poolA}, TokenB=${poolB}, K=${currentK}`);
    
    console.log("\n⚙️ **精确模拟实际代码逻辑：**");
    
    // 步骤1: 计算a2
    const a2 = originalA - wantedA; // 21
    console.log(`  步骤1: a2 = ${originalA} - ${wantedA} = ${a2}`);
    
    // 步骤2: 精确计算amount_in_exact (128位精度)
    // amount_in_exact = (k - a2 * pool_b) / a2
    const k_bigint = BigInt(originalK);
    const a2_bigint = BigInt(a2);
    const poolB_bigint = BigInt(originalB);
    
    const numerator = k_bigint - (a2_bigint * poolB_bigint); // 625 - (21 * 25) = 625 - 525 = 100
    const amount_in_exact_bigint = numerator / a2_bigint; // 100 / 21 = 4 (整数除法)
    const amount_in_exact = Number(amount_in_exact_bigint);
    
    console.log(`  步骤2: numerator = ${originalK} - (${a2} × ${originalB}) = ${Number(numerator)}`);
    console.log(`  步骤3: amount_in_exact = ${Number(numerator)} ÷ ${a2} = ${amount_in_exact}`);
    
    // 步骤3: 计算含手续费的金额 (向上取整)
    const feeRate = 500; // 5%
    const fee_multiplier = 10000 + feeRate; // 10500
    const amount_with_fees_exact_bigint = amount_in_exact_bigint * BigInt(fee_multiplier); // 4 * 10500 = 42000
    
    // 向上取整: ceiling(42000 / 10000) = ceiling(4.2) = 5
    const amount_in_with_fees_bigint = (amount_with_fees_exact_bigint + BigInt(10000 - 1)) / BigInt(10000);
    const amount_in_with_fees = Number(amount_in_with_fees_bigint);
    
    console.log(`  步骤4: 含手续费金额 = ${amount_in_exact} × ${fee_multiplier} = ${Number(amount_with_fees_exact_bigint)}`);
    console.log(`  步骤5: 向上取整 = ceiling(${Number(amount_with_fees_exact_bigint)} ÷ 10000) = ${amount_in_with_fees}`);
    
    console.log("\n🎯 **验证实际结果：**");
    const actualBPaid = poolB - originalB; // 实际付出的TokenB
    console.log(`  理论应付出: ${amount_in_with_fees} TokenB`);
    console.log(`  实际付出: ${actualBPaid} TokenB`);
    
    if (actualBPaid === amount_in_with_fees) {
      console.log(`  ✅ 计算完全正确！手续费收取准确`);
    } else if (actualBPaid > amount_in_with_fees) {
      console.log(`  ⚠️  用户多付了 ${actualBPaid - amount_in_with_fees} TokenB`);
    } else {
      console.log(`  ❌ 用户少付了 ${amount_in_with_fees - actualBPaid} TokenB`);
    }
    
    console.log("\n💰 **K值变化分析：**");
    if (currentK > originalK) {
      const kIncrease = currentK - originalK;
      const increasePercent = (kIncrease / originalK * 100).toFixed(2);
      console.log(`  ✅ K值正确增加: ${originalK} → ${currentK} (增加 ${kIncrease}, +${increasePercent}%)`);
      console.log(`  🎉 手续费成功累积到池子中，LP获得收益!`);
    } else if (currentK === originalK) {
      console.log(`  ⚠️  K值保持不变: ${currentK} (无手续费收入)`);
    } else {
      const kDecrease = originalK - currentK;
      const decreasePercent = (kDecrease / originalK * 100).toFixed(2);
      console.log(`  ❌ K值减少: ${originalK} → ${currentK} (减少 ${kDecrease}, -${decreasePercent}%)`);
      
      if (actualBPaid === amount_in_with_fees) {
        console.log(`  🤔 虽然手续费收取正确，但K值仍减少，可能是整数精度限制`);
      } else {
        console.log(`  🚨 手续费收取不足导致K值减少!`);
      }
    }
    
    console.log("=======================================\n");
  };
});
