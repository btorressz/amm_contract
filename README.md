# amm_contract

This project implements an Automated Market Maker (AMM) on the Solana blockchain using the Anchor framework. The AMM facilitates token swaps and liquidity provision using the constant product formula (x * y = k). Liquidity providers can add and remove liquidity, and users can swap tokens based on the pool's reserves, with a fee distributed to liquidity providers.

## Features
- **Token Swap: Swap tokens using the constant product formula.**
- **Add Liquidity: Add liquidity to the AMM pool in exchange for LP (liquidity provider) shares.**
- **Remove Liquidity: Withdraw liquidity from the pool, adjusting the token reserves and shares.**
- **Fee Distribution: A portion of each swap is collected as a fee and distributed to liquidity providers.**
- **Pause and Unpause: The admin can pause or unpause the contract to prevent interactions during critical updates.**
- **Price Impact Control: Prevents users from receiving fewer tokens than expected during swaps by enforcing a minimum acceptable output.**

