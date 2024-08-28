import BN from "bn.js";
import assert from "assert";
import * as web3 from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import type { AmmContract } from "../target/types/amm_contract";

describe("AMM Contract Tests", () => {
  // Configure the client to use the local cluster
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.AmmContract as anchor.Program<AmmContract>;
  
  let ammAccount: web3.Keypair;
  let mintA: web3.PublicKey;
  let mintB: web3.PublicKey;
  let userAAccount: web3.PublicKey;
  let userBAccount: web3.PublicKey;
  let user: web3.Keypair;
  let mintAuthority: web3.Keypair;

  before(async () => {
    // Initialize accounts and mints before running tests
    ammAccount = new web3.Keypair();
    user = pg.wallet; // Use the default wallet in Solana Playground
    mintAuthority = new web3.Keypair();

    // Airdrop SOL to mintAuthority
    await program.provider.connection.requestAirdrop(mintAuthority.publicKey, web3.LAMPORTS_PER_SOL);

    // Create mints for Token A and Token B
    mintA = await createMint(mintAuthority.publicKey);
    mintB = await createMint(mintAuthority.publicKey);

    // Create token accounts for user to hold Token A and Token B
    userAAccount = await createTokenAccount(mintA, user.publicKey);
    userBAccount = await createTokenAccount(mintB, user.publicKey);

    // Mint tokens to user accounts
    await mintTokens(mintA, userAAccount, mintAuthority, 1000);
    await mintTokens(mintB, userBAccount, mintAuthority, 1000);
  });

  it("Initializes the AMM", async () => {
    const fee = new anchor.BN(30); // 0.3% fee

    // Initialize the AMM with a fee of 30
    const txHash = await program.methods
      .initialize(fee)
      .accounts({
        amm: ammAccount.publicKey,
        user: user.publicKey,
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([ammAccount])
      .rpc();

    console.log(`AMM initialized with transaction: ${txHash}`);

    // Fetch the AMM account to verify initialization
    const amm = await program.account.amm.fetch(ammAccount.publicKey);
    console.log("AMM state after initialization:", amm);

    assert.equal(amm.fee.toNumber(), 30, "AMM fee should be 0.3%");
    assert.equal(amm.tokenAReserve.toNumber(), 0, "Token A reserve should be 0");
    assert.equal(amm.tokenBReserve.toNumber(), 0, "Token B reserve should be 0");
  });

  it("Adds liquidity", async () => {
    const amountA = new anchor.BN(500);
    const amountB = new anchor.BN(500);

    // Add liquidity to the AMM: 500 Token A and 500 Token B
    const txHash = await program.methods
      .addLiquidity(amountA, amountB)
      .accounts({
        amm: ammAccount.publicKey,
        user: user.publicKey,
        userA: userAAccount,
        userB: userBAccount,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .rpc();

    console.log(`Liquidity added with transaction: ${txHash}`);

    // Fetch the AMM account to verify reserves
    const amm = await program.account.amm.fetch(ammAccount.publicKey);
    console.log("AMM state after adding liquidity:", amm);

    assert.equal(amm.tokenAReserve.toNumber(), 500, "Token A reserve should be 500");
    assert.equal(amm.tokenBReserve.toNumber(), 500, "Token B reserve should be 500");
  });

  it("Performs a swap", async () => {
    const amountIn = new anchor.BN(100);
    const minimumOutput = new anchor.BN(90); // Slippage protection
    const fromAtoB = true; // Swap from Token A to Token B

    // Perform the swap
    const txHash = await program.methods
      .swap(amountIn, fromAtoB, minimumOutput)
      .accounts({
        amm: ammAccount.publicKey,
        user: user.publicKey,
        fromAccount: userAAccount,
        toAccount: userBAccount,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .rpc();

    console.log(`Swapped Token A for Token B with transaction: ${txHash}`);

    // Fetch the AMM account to verify updated reserves
    const amm = await program.account.amm.fetch(ammAccount.publicKey);
    console.log("AMM state after swap:", amm);

    assert(amm.tokenAReserve.toNumber() > 500, "Token A reserve should increase");
    assert(amm.tokenBReserve.toNumber() < 500, "Token B reserve should decrease");
  });

  it("Removes liquidity", async () => {
    const shares = new anchor.BN(500);

    // Remove liquidity from the AMM
    const txHash = await program.methods
      .removeLiquidity(shares)
      .accounts({
        amm: ammAccount.publicKey,
        user: user.publicKey,
        userA: userAAccount,
        userB: userBAccount,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .rpc();

    console.log(`Liquidity removed with transaction: ${txHash}`);

    // Fetch the AMM account to verify updated reserves
    const amm = await program.account.amm.fetch(ammAccount.publicKey);
    console.log("AMM state after removing liquidity:", amm);

    assert(amm.tokenAReserve.toNumber() < 500, "Token A reserve should decrease");
    assert(amm.tokenBReserve.toNumber() < 500, "Token B reserve should decrease");
  });

  // Helper functions for mints and token accounts
  async function createMint(mintAuthority: web3.PublicKey): Promise<web3.PublicKey> {
    const mint = new web3.Keypair();
    const tx = new web3.Transaction();
    tx.add(
      web3.SystemProgram.createAccount({
        fromPubkey: program.provider.publicKey,
        newAccountPubkey: mint.publicKey,
        space: anchor.utils.token.MintLayout.span,
        lamports: await anchor.utils.token.getMinBalanceRentForExemptMint(program.provider.connection),
        programId: anchor.utils.token.TOKEN_PROGRAM_ID,
      }),
      anchor.utils.token.createInitializeMintInstruction(
        mint.publicKey,
        6,
        mintAuthority,
        null,
        anchor.utils.token.TOKEN_PROGRAM_ID
      )
    );

    await pg.provider.sendAndConfirm(tx, [mint]);
    return mint.publicKey;
  }

  async function createTokenAccount(mint: web3.PublicKey, owner: web3.PublicKey): Promise<web3.PublicKey> {
    const tokenAccount = new web3.Keypair();
    const tx = new web3.Transaction();
    tx.add(
      web3.SystemProgram.createAccount({
        fromPubkey: program.provider.publicKey,
        newAccountPubkey: tokenAccount.publicKey,
        space: anchor.utils.token.AccountLayout.span,
        lamports: await anchor.utils.token.getMinBalanceRentForExemptAccount(program.provider.connection),
        programId: anchor.utils.token.TOKEN_PROGRAM_ID,
      }),
      anchor.utils.token.createInitializeAccountInstruction(
        tokenAccount.publicKey,
        mint,
        owner,
        anchor.utils.token.TOKEN_PROGRAM_ID
      )
    );

    await pg.provider.sendAndConfirm(tx, [tokenAccount]);
    return tokenAccount.publicKey;
  }

  async function mintTokens(mint: web3.PublicKey, destination: web3.PublicKey, mintAuthority: web3.Keypair, amount: number) {
    const tx = new web3.Transaction();
    tx.add(
      anchor.utils.token.createMintToInstruction(
        mint,
        destination,
        mintAuthority.publicKey,
        amount,
        [],
        anchor.utils.token.TOKEN_PROGRAM_ID
      )
    );

    await pg.provider.sendAndConfirm(tx, [mintAuthority]);
  }
});
