import BN from "bn.js";
import * as web3 from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import type { AmmContract } from "../target/types/amm_contract";

// Configure the client to use the local cluster
anchor.setProvider(anchor.AnchorProvider.env());

const program = anchor.workspace.AmmContract as anchor.Program<AmmContract>;

(async () => {
  try {
    // Log the wallet's public key
    console.log("Wallet address:", program.provider.publicKey.toString());

    // Get and log the wallet's balance in SOL
    const balance = await program.provider.connection.getBalance(program.provider.publicKey);
    console.log(`Wallet balance: ${balance / web3.LAMPORTS_PER_SOL} SOL`);

    // Generate keypairs for the AMM account and mint authority
    const ammAccount = new web3.Keypair();
    const mintAuthority = new web3.Keypair();

    // Airdrop SOL to the mint authority if needed
    console.log("Airdropping SOL to mint authority...");
    const airdropSignature = await program.provider.connection.requestAirdrop(mintAuthority.publicKey, web3.LAMPORTS_PER_SOL);
    await program.provider.connection.confirmTransaction(airdropSignature);

    // Create mint tokens for token A and token B
    const mintA = await createMint(mintAuthority.publicKey);
    const mintB = await createMint(mintAuthority.publicKey);

    // Create token accounts for the user to hold token A and token B
    const user = pg.wallet; // Using the default wallet in the Playground
    const userAAccount = await createTokenAccount(mintA, user.publicKey);
    const userBAccount = await createTokenAccount(mintB, user.publicKey);

    // Mint tokens to the user's token accounts
    console.log("Minting tokens...");
    await mintTokens(mintA, userAAccount, mintAuthority, 1000);
    await mintTokens(mintB, userBAccount, mintAuthority, 1000);

    // Initialize the AMM with a fee of 30 (representing 0.3%)
    console.log("Initializing AMM...");
    const fee = new anchor.BN(30); // 0.3% fee
    await program.methods
      .initialize(fee)
      .accounts({
        amm: ammAccount.publicKey,
        user: user.publicKey,
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([ammAccount])
      .rpc();
    console.log("AMM initialized with fee:", fee.toString());

    // Add liquidity: Adding 500 units of token A and 500 units of token B
    console.log("Adding liquidity...");
    const amountA = new anchor.BN(500);
    const amountB = new anchor.BN(500);
    await program.methods
      .addLiquidity(amountA, amountB)
      .accounts({
        amm: ammAccount.publicKey,
        user: user.publicKey,
        userA: userAAccount,
        userB: userBAccount,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .rpc();
    console.log("Liquidity added: 500 Token A and 500 Token B");

    // Perform a swap: Swap 100 units of token A for token B
    console.log("Performing swap...");
    const amountIn = new anchor.BN(100);
    const minimumOutput = new anchor.BN(90); // Example slippage protection: Require at least 90 units out
    const fromAtoB = true; // Swapping from A to B

    await program.methods
      .swap(amountIn, fromAtoB, minimumOutput)
      .accounts({
        amm: ammAccount.publicKey,
        user: user.publicKey,
        fromAccount: userAAccount,
        toAccount: userBAccount,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .rpc();

    console.log(`Swapped 100 units of Token A for Token B (min output: 90)`);

    // Pause the contract (Admin action)
    console.log("Pausing the contract...");
    await program.methods
      .pauseContract(true)
      .accounts({
        amm: ammAccount.publicKey,
        user: user.publicKey,
      })
      .rpc();
    console.log("Contract paused.");

    // Try removing liquidity while contract is paused (should fail)
    try {
      console.log("Attempting to remove liquidity (while paused)...");
      const shares = new anchor.BN(500);
      await program.methods
        .removeLiquidity(shares)
        .accounts({
          amm: ammAccount.publicKey,
          user: user.publicKey,
          userA: userAAccount,
          userB: userBAccount,
          tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        })
        .rpc();
    } catch (err) {
      console.error("Error removing liquidity while paused:", err);
    }

    // Unpause the contract
    console.log("Unpausing the contract...");
    await program.methods
      .pauseContract(false)
      .accounts({
        amm: ammAccount.publicKey,
        user: user.publicKey,
      })
      .rpc();
    console.log("Contract unpaused.");

    // Remove liquidity: Removing 500 shares
    console.log("Removing liquidity...");
    const shares = new anchor.BN(500);
    await program.methods
      .removeLiquidity(shares)
      .accounts({
        amm: ammAccount.publicKey,
        user: user.publicKey,
        userA: userAAccount,
        userB: userBAccount,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .rpc();
    console.log(`Liquidity removed: 500 shares`);

    // Fetch and log the final state of the AMM account
    const ammState = await program.account.amm.fetch(ammAccount.publicKey);
    console.log("Final AMM state:", ammState);

  } catch (error) {
    console.error("An error occurred:", error);
  }
})();

// Helper functions for creating mints and token accounts

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
