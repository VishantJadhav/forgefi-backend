use anchor_lang::prelude::*;
use anchor_lang::system_program;

// Playground will auto-update this on build
declare_id!("AyN3aAx2VJTSxJGaR5n9Ayhpa6inCAxaSGupxbGw1Rnz");

#[program]
pub mod forgefi {
    use super::*;

    // ==========================================
    // 1. SINGLE PLAYER: INITIALIZE ROUTINE
    // ==========================================
    pub fn initialize_routine(
        ctx: Context<InitializeRoutine>,
        stake_amount: u64,
        days_committed: u16,
    ) -> Result<()> {
        let user_stake = &mut ctx.accounts.user_stake;
        user_stake.user = *ctx.accounts.user.key;
        user_stake.stake_amount = stake_amount;
        user_stake.days_committed = days_committed;

        // --- IRON MATRIX INIT ---
        user_stake.days_completed = 0;
        user_stake.missed_days = 0;
        user_stake.last_check_in = Clock::get()?.unix_timestamp;
        user_stake.bump = ctx.bumps.user_stake;

        // Transfer Devnet SOL from user to the PDA vault to simulate staking
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.user_stake.to_account_info(),
            },
        );
        system_program::transfer(cpi_context, stake_amount)?;

        msg!("ForgeFi Routine locked! Stake deposited: {}", stake_amount);
        Ok(())
    }

    // ==========================================
    // 2. SINGLE PLAYER: VERIFY WORKOUT
    // ==========================================
    pub fn verify_workout(ctx: Context<VerifyWorkout>) -> Result<()> {
        let user_stake = &mut ctx.accounts.user_stake;
        let current_time = Clock::get()?.unix_timestamp;
        let time_since_last = current_time.saturating_sub(user_stake.last_check_in);

        // 1. THE GUILLOTINE CHECK (Closing the Loophole)
        // [DEMO MODE ACTIVATED]: Dropped from 172,800s (48hr) down to 60 seconds
        require!(time_since_last <= 60, ErrorCode::MissedDeadline);

        // 2. THE COOLDOWN CHECK (Anti-Cheat)
        // [DEMO MODE ACTIVATED]: Dropped from 43,200s (12hr) down to 10 seconds
        if user_stake.days_completed > 0 {
            require!(time_since_last >= 10, ErrorCode::WorkoutTooSoon);
        }

        // 3. INCREMENT THE MATRIX
        user_stake.days_completed = user_stake.days_completed.saturating_add(1);
        user_stake.last_check_in = current_time;

        msg!(
            "Workout verified! Iron Matrix updated: {} / {} days completed.",
            user_stake.days_completed,
            user_stake.days_committed
        );
        Ok(())
    }

    // ==========================================
    // 3. SINGLE PLAYER: RESOLVE STAKE
    // ==========================================
    pub fn resolve_stake(ctx: Context<ResolveStake>) -> Result<()> {
        let user_stake = &mut ctx.accounts.user_stake;

        // SECURITY: The protocol ends only when all days are accounted for (verified + missed)
        let total_days_processed = user_stake.days_completed + user_stake.missed_days;
        require!(
            total_days_processed >= user_stake.days_committed,
            ErrorCode::ProtocolNotComplete
        );

        // The `close = user` tag in the struct below automatically sweeps
        // the remaining SOL in the PDA vault directly back to the user.

        msg!("Protocol complete! Surviving stake returned safely. Stay hard.");
        Ok(())
    }

    // ==========================================
    // 4. THE VAMPIRE (Total Liquidation + 10% Bleed)
    // ==========================================
    pub fn slash_missed_day(ctx: Context<SlashUser>) -> Result<()> {
        // --- THE SECURITY LOCK ---
        let official_treasury: Pubkey = "HrAkqgXZA1fkwoJ6tdDcsu84R67yR7KCpB8NUR6oZ5NC"
            .parse()
            .unwrap();
        require!(
            ctx.accounts.treasury.key() == official_treasury,
            ErrorCode::UnauthorizedTreasury
        );

        let user_stake = &mut ctx.accounts.user_stake;
        let current_time = Clock::get()?.unix_timestamp;
        let time_since_last = current_time.saturating_sub(user_stake.last_check_in);

        // THE GUILLOTINE
        // [DEMO MODE ACTIVATED]: Dropped from 172,800s (48hr) down to 60 seconds
        require!(time_since_last > 60, ErrorCode::DeadlineNotPassed);

        // Calculate exactly 10% of their initial locked amount
        let penalty_amount = user_stake.stake_amount / 10;

        let current_balance = user_stake.to_account_info().lamports();
        let rent_minimum = Rent::get()?.minimum_balance(user_stake.to_account_info().data_len());

        // Calculate what we can actually take without deleting the account
        let available_to_slash = current_balance.saturating_sub(rent_minimum);

        // --- THE TOTAL LIQUIDATION CHECK ---
        if available_to_slash < penalty_amount {
            // Sweep all available SOL, but leave the rent alive so the account data survives
            user_stake
                .to_account_info()
                .sub_lamports(available_to_slash)?;
            ctx.accounts
                .treasury
                .to_account_info()
                .add_lamports(available_to_slash)?;

            // Flag as a Zombie Vault
            user_stake.missed_days = 999;
            user_stake.last_check_in = current_time;

            msg!("TOTAL LIQUIDATION: Vault drained. Zombie state triggered.");
            return Ok(());
        }

        // --- NORMAL 10% SLASH ---
        user_stake.to_account_info().sub_lamports(penalty_amount)?;
        ctx.accounts
            .treasury
            .to_account_info()
            .add_lamports(penalty_amount)?;

        // Record the failure and reset the 60-second clock for their next attempt
        user_stake.missed_days = user_stake.missed_days.saturating_add(1);
        user_stake.last_check_in = current_time;

        msg!(
            "SLASHED: Lifter missed a day. {} lamports bled to the Graveyard.",
            penalty_amount
        );
        Ok(())
    }

    // ==========================================
    // 4.5. ACKNOWLEDGE FAILURE (Burn the Zombie)
    // ==========================================
    pub fn acknowledge_failure(_ctx: Context<AcknowledgeFailure>) -> Result<()> {
        // The `close = user` constraint does all the work.
        // It burns the account and returns the rent dust to the user.
        msg!("Lifter acknowledged failure. Zombie vault burned. Slate wiped clean.");
        Ok(())
    }

    // ==========================================
    // 5. THE BLOOD PACT: INITIALIZE SQUAD LOBBY
    // ==========================================
    pub fn initialize_squad(
        ctx: Context<InitializeSquad>,
        required_stake: u64,
        days: u16,
        player_two: Pubkey,
        player_three: Pubkey, // Pass the System Program ID if it's just a duo
    ) -> Result<()> {
        let vault = &mut ctx.accounts.squad_vault;
        let player_one = &ctx.accounts.player_one;

        // 1. Setup the Database
        vault.player_one = player_one.key();
        vault.player_two = player_two;
        vault.player_three = player_three;
        vault.required_stake_per_player = required_stake;
        vault.days_committed = days;

        vault.days_completed = 0;
        vault.missed_days = 0;
        vault.bump = ctx.bumps.squad_vault;

        // 2. Player 1 Locks their SOL via CPI
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: player_one.to_account_info(),
                to: vault.to_account_info(),
            },
        );
        system_program::transfer(cpi_context, required_stake)?;

        // 3. Update the Lobby State
        vault.total_vault_balance = required_stake;
        vault.p1_staked = true;
        vault.p2_staked = false;
        vault.p3_staked = false;
        vault.protocol_active = false; // Waiting on the others

        msg!(
            "Blood Pact forged. Player 1 locked {} lamports. Awaiting squad...",
            required_stake
        );
        Ok(())
    }

    // ==========================================
    // 6. THE BLOOD PACT: JOIN SQUAD
    // ==========================================
    pub fn join_squad(ctx: Context<JoinSquad>) -> Result<()> {
        let vault = &mut ctx.accounts.squad_vault;
        let joining_player = &ctx.accounts.player;

        // 1. Verify this person is actually invited
        require!(
            joining_player.key() == vault.player_two || joining_player.key() == vault.player_three,
            ErrorCode::NotInvited
        );

        // 2. Transfer their SOL into the Vault via CPI
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: joining_player.to_account_info(),
                to: vault.to_account_info(),
            },
        );
        system_program::transfer(cpi_context, vault.required_stake_per_player)?;

        // 3. Mark them as Staked
        if joining_player.key() == vault.player_two {
            vault.p2_staked = true;
        } else if joining_player.key() == vault.player_three {
            vault.p3_staked = true;
        }

        vault.total_vault_balance += vault.required_stake_per_player;

        // 4. THE ACTIVATION TRIGGER
        let is_duo = vault.player_three == anchor_lang::solana_program::system_program::ID;

        if (is_duo && vault.p1_staked && vault.p2_staked)
            || (!is_duo && vault.p1_staked && vault.p2_staked && vault.p3_staked)
        {
            vault.protocol_active = true;

            // Start the clocks!
            let current_time = Clock::get()?.unix_timestamp;
            vault.p1_last_check_in = current_time;
            vault.p2_last_check_in = current_time;
            vault.p3_last_check_in = current_time;

            msg!("ALL PLAYERS LOCKED. THE BLOOD PACT IS ACTIVE.");
        } else {
            msg!("Player locked SOL. Waiting for remaining members...");
        }

        Ok(())
    }
}

// --- DATABASE SCHEMA ---
#[account]
pub struct UserStake {
    pub user: Pubkey,
    pub stake_amount: u64,
    pub days_committed: u16,
    pub days_completed: u16,
    pub missed_days: u16,
    pub last_check_in: i64,
    pub bump: u8,
}

#[account]
pub struct SquadVault {
    pub player_one: Pubkey,
    pub player_two: Pubkey,
    pub player_three: Pubkey,
    pub required_stake_per_player: u64,
    pub total_vault_balance: u64,
    pub days_committed: u16,
    pub days_completed: u16,
    pub missed_days: u16,
    pub p1_staked: bool,
    pub p2_staked: bool,
    pub p3_staked: bool,
    pub protocol_active: bool,
    pub p1_last_check_in: i64,
    pub p2_last_check_in: i64,
    pub p3_last_check_in: i64,
    pub bump: u8,
}

// --- VALIDATION BOUNCERS ---
#[derive(Accounts)]
pub struct InitializeRoutine<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init,
        payer = user,
        space = 8 + 32 + 8 + 2 + 2 + 2 + 8 + 1, 
        seeds = [b"stake", user.key().as_ref()], 
        bump
    )]
    pub user_stake: Account<'info, UserStake>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct VerifyWorkout<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"stake", user.key().as_ref()], 
        bump = user_stake.bump
    )]
    pub user_stake: Account<'info, UserStake>,
}

#[derive(Accounts)]
pub struct ResolveStake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        close = user, 
        seeds = [b"stake", user.key().as_ref()], 
        bump = user_stake.bump
    )]
    pub user_stake: Account<'info, UserStake>,
}

#[derive(Accounts)]
pub struct SlashUser<'info> {
    #[account(mut)]
    pub liquidator: Signer<'info>,

    /// The protocol's Graveyard wallet where slashed SOL is deposited
    #[account(mut)]
    pub treasury: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"stake", user_stake.user.as_ref()], 
        bump = user_stake.bump
    )]
    pub user_stake: Account<'info, UserStake>,
}

#[derive(Accounts)]
pub struct AcknowledgeFailure<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        close = user, 
        seeds = [b"stake", user.key().as_ref()], 
        bump = user_stake.bump,
        constraint = user_stake.missed_days == 999 @ ErrorCode::NotAZombie
    )]
    pub user_stake: Account<'info, UserStake>,
}

#[derive(Accounts)]
pub struct InitializeSquad<'info> {
    #[account(mut)]
    pub player_one: Signer<'info>,
    #[account(
        init,
        payer = player_one,
        space = 8 + 200, // Safe padding for all pubkeys, u64s, and booleans
        seeds = [b"squad", player_one.key().as_ref()], 
        bump
    )]
    pub squad_vault: Account<'info, SquadVault>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct JoinSquad<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(mut)]
    pub squad_vault: Account<'info, SquadVault>,
    pub system_program: Program<'info, System>,
}

// --- CUSTOM ERROR CODES ---
#[error_code]
pub enum ErrorCode {
    #[msg("Muscles need rest. You must wait at least 10 seconds between verified workouts.")]
    WorkoutTooSoon,
    #[msg("Protocol not complete. You cannot withdraw your stake until all committed days are processed.")]
    ProtocolNotComplete,
    #[msg("The guillotine has not dropped yet. The lifter still has time in their window.")]
    DeadlineNotPassed,
    #[msg("You missed your window. Your stake is bleeding and awaiting liquidation.")]
    MissedDeadline,
    #[msg("Security Alert: Slashed funds can only be routed to the official ForgeFi Treasury.")]
    UnauthorizedTreasury,
    #[msg("You are not invited to this Blood Pact.")]
    NotInvited,
    #[msg("You cannot burn this vault. It is not a Zombie.")]
    NotAZombie,
}
