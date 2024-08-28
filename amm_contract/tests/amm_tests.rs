u  // Configure the client to use the local cluster
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.AmmContract as anchor.Program<AmmContract>;
  
import type { AmmContract } from "../target/types/amm_contract";
se anchor_lang::prelude::*;
use anchor_lang::prelude::Pubkey;
use anchor_spl::token::{self, TokenAccount, Transfer, Mint, Token};
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
    transport::TransportError,
};
use amm_contract::*; // Replace with your module name if different

#[tokio::test]
async fn test_initialize() -> Result<(), TransportError> {
    let program = ProgramTest::new(
        "amm_contract", // Updated program name
        id(),           // Replace with your program's ID
        processor!(amm_contract::entry), // Updated entry function
    );

    let (mut banks_client, payer, recent_blockhash) = program.start().await;

    let amm_account = Keypair::new();
    let user = Keypair::new();

    // Initialize the AMM
    let tx = Transaction::new_signed_with_payer(
        &[instruction::initialize(
            &id(),
            &amm_account.pubkey(),
            &user.pubkey(),
            &payer.pubkey(),
            30, // Fee
        )],
        Some(&payer.pubkey()),
        &[&payer, &amm_account, &user],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await.unwrap();

    // Fetch and verify the AMM account state
    let amm_account_info = banks_client
        .get_account(amm_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let amm: Amm = Amm::try_from_slice(&amm_account_info.data).unwrap();
    assert_eq!(amm.fee, 30);
    assert_eq!(amm.token_a_reserve, 0);
    assert_eq!(amm.token_b_reserve, 0);

    Ok(())
}

#[tokio::test]
async fn test_add_liquidity() -> Result<(), TransportError> {
    let program = ProgramTest::new(
        "amm_contract", // Updated program name
        id(),
        processor!(amm_contract::entry), // Updated entry function
    );

    let (mut banks_client, payer, recent_blockhash) = program.start().await;

    let amm_account = Keypair::new();
    let user = Keypair::new();
    let user_a_account = Keypair::new();
    let user_b_account = Keypair::new();

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();

    // Create token mints and user accounts
    create_mint(&mut banks_client, &payer, &recent_blockhash, &mint_a, &user.pubkey()).await?;
    create_mint(&mut banks_client, &payer, &recent_blockhash, &mint_b, &user.pubkey()).await?;

    create_token_account(&mut banks_client, &payer, &recent_blockhash, &user_a_account, &mint_a.pubkey(), &user.pubkey()).await?;
    create_token_account(&mut banks_client, &payer, &recent_blockhash, &user_b_account, &mint_b.pubkey(), &user.pubkey()).await?;

    mint_tokens(&mut banks_client, &payer, &recent_blockhash, &mint_a.pubkey(), &user_a_account.pubkey(), &user, 1000).await?;
    mint_tokens(&mut banks_client, &payer, &recent_blockhash, &mint_b.pubkey(), &user_b_account.pubkey(), &user, 1000).await?;

    // Initialize the AMM
    let tx = Transaction::new_signed_with_payer(
        &[instruction::initialize(
            &id(),
            &amm_account.pubkey(),
            &user.pubkey(),
            &payer.pubkey(),
            30,
        )],
        Some(&payer.pubkey()),
        &[&payer, &amm_account, &user],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await.unwrap();

    // Add liquidity
    let tx = Transaction::new_signed_with_payer(
        &[instruction::add_liquidity(
            &id(),
            &amm_account.pubkey(),
            &user.pubkey(),
            &user_a_account.pubkey(),
            &user_b_account.pubkey(),
            500, // Amount A
            500, // Amount B
        )],
        Some(&payer.pubkey()),
        &[&payer, &user],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await.unwrap();

    // Fetch and verify the AMM account state
    let amm_account_info = banks_client
        .get_account(amm_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let amm: Amm = Amm::try_from_slice(&amm_account_info.data).unwrap();
    assert_eq!(amm.token_a_reserve, 500);
    assert_eq!(amm.token_b_reserve, 500);
    assert_eq!(amm.total_shares, 1000);

    Ok(())
}

#[tokio::test]
async fn test_swap() -> Result<(), TransportError> {
    let program = ProgramTest::new(
        "amm_contract", // program name
        id(),
        processor!(amm_contract::entry), // entry function
    );

    let (mut banks_client, payer, recent_blockhash) = program.start().await;

    let amm_account = Keypair::new();
    let user = Keypair::new();
    let user_a_account = Keypair::new();
    let user_b_account = Keypair::new();

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();

    // Create token mints and user accounts
    create_mint(&mut banks_client, &payer, &recent_blockhash, &mint_a, &user.pubkey()).await?;
    create_mint(&mut banks_client, &payer, &recent_blockhash, &mint_b, &user.pubkey()).await?;

    create_token_account(&mut banks_client, &payer, &recent_blockhash, &user_a_account, &mint_a.pubkey(), &user.pubkey()).await?;
    create_token_account(&mut banks_client, &payer, &recent_blockhash, &user_b_account, &mint_b.pubkey(), &user.pubkey()).await?;

    mint_tokens(&mut banks_client, &payer, &recent_blockhash, &mint_a.pubkey(), &user_a_account.pubkey(), &user, 1000).await?;
    mint_tokens(&mut banks_client, &payer, &recent_blockhash, &mint_b.pubkey(), &user_b_account.pubkey(), &user, 1000).await?;

    // Initialize the AMM
    let tx = Transaction::new_signed_with_payer(
        &[instruction::initialize(
            &id(),
            &amm_account.pubkey(),
            &user.pubkey(),
            &payer.pubkey(),
            30,
        )],
        Some(&payer.pubkey()),
        &[&payer, &amm_account, &user],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await.unwrap();

    // Add liquidity
    let tx = Transaction::new_signed_with_payer(
        &[instruction::add_liquidity(
            &id(),
            &amm_account.pubkey(),
            &user.pubkey(),
            &user_a_account.pubkey(),
            &user_b_account.pubkey(),
            500, // Amount A
            500, // Amount B
        )],
        Some(&payer.pubkey()),
        &[&payer, &user],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await.unwrap();

    // Perform swap
    let tx = Transaction::new_signed_with_payer(
        &[instruction::swap(
            &id(),
            &amm_account.pubkey(),
            &user.pubkey(),
            &user_a_account.pubkey(),
            &user_b_account.pubkey(),
            100, // Amount in
            true, // From A to B
        )],
        Some(&payer.pubkey()),
        &[&payer, &user],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await.unwrap();

    // Fetch and verify the AMM account state
    let amm_account_info = banks_client
        .get_account(amm_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let amm: Amm = Amm::try_from_slice(&amm_account_info.data).unwrap();
    assert!(amm.token_a_reserve > 500);
    assert!(amm.token_b_reserve < 500);

    Ok(())
}

#[tokio::test]
async fn test_remove_liquidity() -> Result<(), TransportError> {
    let program = ProgramTest::new(
        "amm_contract", //  program name
        id(),
        processor!(amm_contract::entry), //  entry function
    );

    let (mut banks_client, payer, recent_blockhash) = program.start().await;

    let amm_account = Keypair::new();
    let user = Keypair::new();
    let user_a_account = Keypair::new();
    let user_b_account = Keypair::new();

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();

    // Create token mints and user accounts
    create_mint(&mut banks_client, &payer, &recent_blockhash, &mint_a, &user.pubkey()).await?;
    create_mint(&mut banks_client, &payer, &recent_blockhash, &mint_b, &user.pubkey()).await?;

    create_token_account(&mut banks_client, &payer, &recent_blockhash, &user_a_account, &mint_a.pubkey(), &user.pubkey()).await?;
    create_token_account(&mut banks_client, &payer, &recent_blockhash, &user_b_account, &mint_b.pubkey(), &user.pubkey()).await?;

    mint_tokens(&mut banks_client, &payer, &recent_blockhash, &mint_a.pubkey(), &user_a_account.pubkey(), &user, 1000).await?;
    mint_tokens(&mut banks_client, &payer, &recent_blockhash, &mint_b.pubkey(), &user_b_account.pubkey(), &user, 1000).await?;

    // Initialize the AMM
    let tx = Transaction::new_signed_with_payer(
        &[instruction::initialize(
            &id(),
            &amm_account.pubkey(),
            &user.pubkey(),
            &payer.pubkey(),
            30,
        )],
        Some(&payer.pubkey()),
        &[&payer, &amm_account, &user],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await.unwrap();

    // Add liquidity
    let tx = Transaction::new_signed_with_payer(
        &[instruction::add_liquidity(
            &id(),
            &amm_account.pubkey(),
            &user.pubkey(),
            &user_a_account.pubkey(),
            &user_b_account.pubkey(),
            500, // Amount A
            500, // Amount B
        )],
        Some(&payer.pubkey()),
        &[&payer, &user],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await.unwrap();

    // Remove liquidity
    let tx = Transaction::new_signed_with_payer(
        &[instruction::remove_liquidity(
            &id(),
            &amm_account.pubkey(),
            &user.pubkey(),
            &user_a_account.pubkey(),
            &user_b_account.pubkey(),
            500, // Shares
        )],
        Some(&payer.pubkey()),
        &[&payer, &user],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await.unwrap();

    // Fetch and verify the AMM account state
    let amm_account_info = banks_client
        .get_account(amm_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let amm: Amm = Amm::try_from_slice(&amm_account_info.data).unwrap();
    assert!(amm.token_a_reserve < 500);
    assert!(amm.token_b_reserve < 500);
    assert_eq!(amm.total_shares, 500);

    Ok(())
}

// Helper functions for creating mints, token accounts, and minting tokens directly within this file

async fn create_mint(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &solana_sdk::hash::Hash,
    mint: &Keypair,
    mint_authority: &Pubkey,
) -> Result<(), TransportError> {
    let tx = Transaction::new_signed_with_payer(
        &[
            solana_sdk::system_instruction::create_account(
                &payer.pubkey(),
                &mint.pubkey(),
                1_000_000_000, // Rent-exempt amount for mint account
                Mint::LEN as u64,
                &token::ID,
            ),
            token::instruction::initialize_mint(
                &token::ID,
                &mint.pubkey(),
                mint_authority,
                None,
                6, // Decimals
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
        &[payer, mint],
        *recent_blockhash,
    );
    banks_client.process_transaction(tx).await
}

async fn create_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &solana_sdk::hash::Hash,
    token_account: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Result<(), TransportError> {
    let tx = Transaction::new_signed_with_payer(
        &[
            solana_sdk::system_instruction::create_account(
                &payer.pubkey(),
                &token_account.pubkey(),
                1_000_000_000, // Rent-exempt amount for token account
                TokenAccount::LEN as u64,
                &token::ID,
            ),
            token::instruction::initialize_account(
                &token::ID,
                &token_account.pubkey(),
                mint,
                owner,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
        &[payer, token_account],
        *recent_blockhash,
    );
    banks_client.process_transaction(tx).await
}

async fn mint_tokens(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &solana_sdk::hash::Hash,
    mint: &Pubkey,
    destination: &Pubkey,
    mint_authority: &Keypair,
    amount: u64,
) -> Result<(), TransportError> {
    let tx = Transaction::new_signed_with_payer(
        &[
            token::instruction::mint_to(
                &token::ID,
                mint,
                destination,
                &mint_authority.pubkey(),
                &[],
                amount,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
        &[payer, mint_authority],
        *recent_blockhash,
    );
    banks_client.process_transaction(tx).await
}
