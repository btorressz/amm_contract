use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer, Mint, Token};
//use anchor_lang::solana_program::program_error::ProgramError;

// Program ID
declare_id!("HYtYWSrCFTumBJDfzJmBqCuBCPx5brMmtnV4b3qYzQyr");

#[program]
mod amm_contract {
    use super::*;

    // Initialize the AMM
    pub fn initialize(ctx: Context<Initialize>, fee: u64) -> Result<()> {
        let amm = &mut ctx.accounts.amm;
        amm.token_a_reserve = 0;
        amm.token_b_reserve = 0;
        amm.fee = fee; // Set the fee (e.g., 30 for 0.3%)
        amm.total_shares = 0;
        amm.accumulated_fees_a = 0;
        amm.accumulated_fees_b = 0;
        amm.paused = false; // Start in unpaused state
        Ok(())
    }

    // Add liquidity to the AMM
    pub fn add_liquidity(ctx: Context<AddLiquidity>, amount_a: u64, amount_b: u64) -> Result<()> {
        let amm = &mut ctx.accounts.amm;

        // Input validation
        if amount_a == 0 || amount_b == 0 {
            return Err(ErrorCode::InvalidInput.into());
        }

        // Calculate shares to mint
        let shares = if amm.total_shares == 0 {
            amount_a + amount_b // Initial liquidity, 1:1 ratio
        } else {
            let share_a = (amount_a * amm.total_shares) / amm.token_a_reserve;
            let share_b = (amount_b * amm.total_shares) / amm.token_b_reserve;
            share_a.min(share_b)
        };

        // Update reserves and total shares
        amm.token_a_reserve += amount_a;
        amm.token_b_reserve += amount_b;
        amm.total_shares += shares;

        // Transfer tokens to AMM
        let cpi_accounts_a = Transfer {
            from: ctx.accounts.user_a.to_account_info(),
            to: ctx.accounts.token_a_reserve_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx_a = CpiContext::new(cpi_program.clone(), cpi_accounts_a);
        token::transfer(cpi_ctx_a, amount_a)?;

        let cpi_accounts_b = Transfer {
            from: ctx.accounts.user_b.to_account_info(),
            to: ctx.accounts.token_b_reserve_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_ctx_b = CpiContext::new(cpi_program, cpi_accounts_b);
        token::transfer(cpi_ctx_b, amount_b)?;

        // Emit an event for adding liquidity
        emit!(AddLiquidityEvent {
            user: ctx.accounts.user.key(),
            amount_a,
            amount_b,
            shares,
        });

        Ok(())
    }

    // Swap function using the constant product formula
    pub fn swap(ctx: Context<Swap>, amount_in: u64, from_a_to_b: bool, minimum_output: u64) -> Result<()> {
        let amm = &mut ctx.accounts.amm;

        // Check if the contract is paused
        if amm.paused {
            return Err(ErrorCode::ContractPaused.into());
        }

        // Input validation
        if amount_in == 0 {
            return Err(ErrorCode::InvalidInput.into());
        }

        let (token_in_reserve, token_out_reserve, accumulated_fees) = if from_a_to_b {
            let token_in_reserve = amm.token_a_reserve;
            let token_out_reserve = amm.token_b_reserve;
            let accumulated_fees = amm.accumulated_fees_b;
            (token_in_reserve, token_out_reserve, accumulated_fees)
        } else {
            let token_in_reserve = amm.token_b_reserve;
            let token_out_reserve = amm.token_a_reserve;
            let accumulated_fees = amm.accumulated_fees_a;
            (token_in_reserve, token_out_reserve, accumulated_fees)
        };

        // Calculate the amount out using the constant product formula
        let amount_out = calculate_amount_out(amount_in, token_in_reserve, token_out_reserve, amm.fee)?;

        // Slippage protection: ensure the amount out is greater than or equal to the minimum output
        if amount_out < minimum_output {
            return Err(ErrorCode::SlippageExceeded.into());
        }

        // Collect fees
        let fee_amount = amount_in * amm.fee / 1000;
        if from_a_to_b {
            amm.accumulated_fees_b += fee_amount;
            amm.token_a_reserve += amount_in - fee_amount;
            amm.token_b_reserve -= amount_out;
        } else {
            amm.accumulated_fees_a += fee_amount;
            amm.token_b_reserve += amount_in - fee_amount;
            amm.token_a_reserve -= amount_out;
        }

        // Perform the token transfer
        let cpi_accounts = Transfer {
            from: ctx.accounts.from_account.to_account_info(),
            to: ctx.accounts.to_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount_out)?;

        // Emit an event for the swap
        emit!(SwapEvent {
            user: ctx.accounts.user.key(),
            amount_in,
            amount_out,
            from_a_to_b,
        });

        Ok(())
    }

    // Remove liquidity from the AMM
    pub fn remove_liquidity(ctx: Context<RemoveLiquidity>, shares: u64) -> Result<()> {
        let amm = &mut ctx.accounts.amm;

        // Calculate the amount of tokens to return
        let amount_a = shares * amm.token_a_reserve / amm.total_shares;
        let amount_b = shares * amm.token_b_reserve / amm.total_shares;

        // Update reserves and total shares
        amm.token_a_reserve -= amount_a;
        amm.token_b_reserve -= amount_b;
        amm.total_shares -= shares;

        // Transfer tokens back to user
        let cpi_accounts_a = Transfer {
            from: ctx.accounts.token_a_reserve_account.to_account_info(),
            to: ctx.accounts.user_a.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx_a = CpiContext::new(cpi_program.clone(), cpi_accounts_a);
        token::transfer(cpi_ctx_a, amount_a)?;

        let cpi_accounts_b = Transfer {
            from: ctx.accounts.token_b_reserve_account.to_account_info(),
            to: ctx.accounts.user_b.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_ctx_b = CpiContext::new(cpi_program, cpi_accounts_b);
        token::transfer(cpi_ctx_b, amount_b)?;

        // Emit an event for removing liquidity
        emit!(RemoveLiquidityEvent {
            user: ctx.accounts.user.key(),
            amount_a,
            amount_b,
            shares,
        });

        Ok(())
    }

    // Distribute accumulated fees to liquidity providers
    pub fn distribute_fees(ctx: Context<DistributeFees>) -> Result<()> {
        let amm = &mut ctx.accounts.amm;

        // Transfer accumulated fees to fee_receiver
        let cpi_accounts_a = Transfer {
            from: ctx.accounts.fee_reserve_a.to_account_info(),
            to: ctx.accounts.fee_receiver.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx_a = CpiContext::new(cpi_program.clone(), cpi_accounts_a);
        token::transfer(cpi_ctx_a, amm.accumulated_fees_a)?;

        let cpi_accounts_b = Transfer {
            from: ctx.accounts.fee_reserve_b.to_account_info(),
            to: ctx.accounts.fee_receiver.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_ctx_b = CpiContext::new(cpi_program, cpi_accounts_b);
        token::transfer(cpi_ctx_b, amm.accumulated_fees_b)?;

        // Reset accumulated fees
        amm.accumulated_fees_a = 0;
        amm.accumulated_fees_b = 0;

        // Emit fee distribution event
        emit!(FeeDistributedEvent {
            user: ctx.accounts.user.key(),
            amount_a: amm.accumulated_fees_a,
            amount_b: amm.accumulated_fees_b,
        });

        Ok(())
    }

    // Pause or unpause the contract
    pub fn pause_contract(ctx: Context<AdminAction>, paused: bool) -> Result<()> {
        let amm = &mut ctx.accounts.amm;
        amm.paused = paused;
        Ok(())
    }
}

// Helper function to calculate the output amount based on the constant product formula
fn calculate_amount_out(amount_in: u64, reserve_in: u64, reserve_out: u64, fee: u64) -> Result<u64> {
    let amount_in_with_fee = amount_in * (1000 - fee);
    let numerator = amount_in_with_fee * reserve_out;
    let denominator = reserve_in * 1000 + amount_in_with_fee;
    Ok(numerator / denominator)
}

// AMM struct
#[account]
pub struct Amm {
    pub token_a_reserve: u64,
    pub token_b_reserve: u64,
    pub fee: u64,
    pub total_shares: u64,
    pub accumulated_fees_a: u64,
    pub accumulated_fees_b: u64,
    pub paused: bool, // Contract paused state
}

// Context for Initialize function
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 64)]
    pub amm: Account<'info, Amm>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

// Context for Swap function
#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub amm: Account<'info, Amm>,
    #[account(mut)]
    pub from_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub to_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

// Context for AddLiquidity function
#[derive(Accounts)]
pub struct AddLiquidity<'info> {
    #[account(mut)]
    pub amm: Account<'info, Amm>,
    #[account(mut)]
    pub token_a_reserve_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub token_b_reserve_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_a: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_b: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

// Context for RemoveLiquidity function
#[derive(Accounts)]
pub struct RemoveLiquidity<'info> {
    #[account(mut)]
    pub amm: Account<'info, Amm>,
    #[account(mut)]
    pub token_a_reserve_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub token_b_reserve_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_a: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_b: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

// Context for DistributeFees function
#[derive(Accounts)]
pub struct DistributeFees<'info> {
    #[account(mut)]
    pub amm: Account<'info, Amm>,
    #[account(mut)]
    pub fee_reserve_a: Account<'info, TokenAccount>,
    #[account(mut)]
    pub fee_reserve_b: Account<'info, TokenAccount>,
    #[account(mut)]
    pub fee_receiver: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

// Context for Admin actions (pausing contract)
#[derive(Accounts)]
pub struct AdminAction<'info> {
    #[account(mut)]
    pub amm: Account<'info, Amm>,
    pub user: Signer<'info>,
}

// Custom error codes for better error handling
#[error_code]
pub enum ErrorCode {
    #[msg("Invalid input.")]
    InvalidInput,
    #[msg("Slippage exceeded the allowed limit.")]
    SlippageExceeded,
    #[msg("The contract is paused.")]
    ContractPaused,
}

// Events
#[event]
pub struct SwapEvent {
    pub user: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub from_a_to_b: bool,
}

#[event]
pub struct AddLiquidityEvent {
    pub user: Pubkey,
    pub amount_a: u64,
    pub amount_b: u64,
    pub shares: u64,
}

#[event]
pub struct RemoveLiquidityEvent {
    pub user: Pubkey,
    pub amount_a: u64,
    pub amount_b: u64,
    pub shares: u64,
}

#[event]
pub struct FeeDistributedEvent {
    pub user: Pubkey,
    pub amount_a: u64,
    pub amount_b: u64,
}
