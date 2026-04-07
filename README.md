# ⚙️ ForgeFi: Backend & Smart Contracts

> **🏆 Frontier Hackathon Submission Note:**
> The core `userStake` PDA contract was established prior to this hackathon (see `pre-frontier-baseline` tag). 
> 
> **For the Frontier Hackathon, we are specifically building these on-chain upgrades in this repository:**
> * **Multiplayer Carnage:** Expanding PDAs to support Squad Vaults for shared financial accountability.
> * **Yield-Bearing Iron:** Integrating Liquid Staking Tokens (LSTs) so locked iron earns yield.
> * **The Live Graveyard:** Emitting optimized on-chain logs for the frontend to catch and render in real-time.
> * **Decentralized Crank:** Transitioning the Executioner bot to a fully permissionless, decentralized automation network.
> * **Zero-Knowledge Geolocation:** Verifying physical gym presence without doxxing user coordinates.

---

### The Immutable Arbiter

This repository contains the Solana smart contracts for **ForgeFi**, built using the [Anchor Framework](https://www.anchor-lang.com/). It acts as the decentralized enforcer of the protocol. It holds the logic for creating user vaults, tracking the 48-hour time windows, and executing the mathematical consequences of missing a workout. 

### 🏗️ Architecture

The contract is designed around a core Program Derived Address (PDA) called the `UserStake`. 

When a user commits to a ForgeFi routine, they transfer SOL into this PDA. The contract enforces strict mathematical rules on this locked capital:
1. **The Lockup:** Users cannot withdraw their principal until their committed days are successfully completed.
2. **The Time Window:** The `last_check_in` timestamp must be updated every 48 hours.
3. **The Vampire Bleed:** If the 48-hour window expires, any external wallet acting as a liquidator (our Executioner Bot) can permissionlessly call the `slash_missed_day` instruction. The contract calculates exactly 10% of the vault and bleeds it to the protocol treasury.

### 📜 Core Instructions

* `initialize_stake`: Derives the PDA and locks the user's initial SOL commitment.
* `verify_workout`: Updates the `last_check_in` timestamp (requires cryptographic verification from the frontend Geolocation Oracle).
* `slash_missed_day`: The liquidation function. Validates the time delta and forcefully routes 10% of the vault to the ForgeFi Treasury.
* `claim_victory`: Unlocks the remaining vault balance and returns it to the user *only* if `days_completed` equals `days_committed`.

### 🛠️ Local Development

To build and test the contract locally:

```bash
# Install dependencies
yarn install

# Build the Anchor program
anchor build

# Run the local test suite
anchor test
