import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Amm } from "../target/types/amm";
import { Keypair, LAMPORTS_PER_SOL, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import { BN, min } from "bn.js";
import { ASSOCIATED_PROGRAM_ID, TOKEN_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
import { createAssociatedTokenAccountIdempotentInstruction, createInitializeMint2Instruction, createMintToInstruction, getAssociatedTokenAddressSync, getMinimumBalanceForRentExemptMint, MINT_SIZE } from "@solana/spl-token";

describe("amm", () => {
  anchor.setProvider(anchor.AnchorProvider.env());

  const provider = anchor.getProvider();

  const connection = provider.connection;

  const program = anchor.workspace.amm as Program<Amm>;

  const confirm = async (signature: string): Promise<string> => {
    const block = await connection.getLatestBlockhash();
    await connection.confirmTransaction({
      signature,
      ...block,
    });
    return signature;
  };

  const log = async (signature: string): Promise<string> => {
    console.log(
      `Your transaction signature: https://explorer.solana.com/transaction/${signature}?cluster=custom&customUrl=${connection.rpcEndpoint}`
    );
    return signature;
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
    let lamports = await getMinimumBalanceForRentExemptMint(connection);
    let tx = new Transaction();
    tx.instructions = [
      SystemProgram.transfer({
        fromPubkey: provider.publicKey,
        toPubkey: signer.publicKey,
        lamports: 10 * LAMPORTS_PER_SOL,
      }),
      ...[mintA, mintB].map((mint) =>
        SystemProgram.createAccount({
          fromPubkey: provider.publicKey,
          newAccountPubkey: mint.publicKey,
          lamports,
          space: MINT_SIZE,
          programId: tokenProgram,
        })
      ),
      createInitializeMint2Instruction(mintA.publicKey, 6, provider.publicKey!, null, tokenProgram),
      createInitializeMint2Instruction(mintB.publicKey, 6, provider.publicKey!, null, tokenProgram),
      createAssociatedTokenAccountIdempotentInstruction(provider.publicKey, signerAtaA, signer.publicKey, mintA.publicKey, tokenProgram),
      createAssociatedTokenAccountIdempotentInstruction(provider.publicKey, signerAtaB, signer.publicKey, mintB.publicKey, tokenProgram),
      createMintToInstruction(mintA.publicKey, signerAtaA, provider.publicKey!, 1e9, undefined, tokenProgram),
      createMintToInstruction(mintB.publicKey, signerAtaB, provider.publicKey!, 1e9, undefined, tokenProgram),
    ];

    await provider.sendAndConfirm(tx, [mintA, mintB]).then(log);
  });

  it("Initialize a pool", async () => {
    // Add your test here.
    const tx = await program.methods.initialize(
      fee.toNumber()
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
  });

  it("Swap", async () => {
    const tx = await program.methods.swap(
      new BN(4), new BN(5), true
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
  });

  it("Withdraw", async () => {
    const tx = await program.methods.deposit(
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
  });
});
