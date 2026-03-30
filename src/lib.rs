use anchor_lang::prelude::*;
use anchor_lang::system_program;

// Playground will auto-update this on build
declare_id!("AyN3aAx2VJTSxJGaR5n9Ayhpa6inCAxaSGupxbGw1Rnz");

#[program]
pub mod forgefi {
    use super::*;

    // 1. Lock in the routine and transfer the stake to the PDA vault
    pub fn initialize_routine(
        ctx: Context<InitializeRoutine>,
        stake_amount: u64,
        days_committed: u8,
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

    // 2. Daily check-in
    pub fn verify_workout(ctx: Context<VerifyWorkout>) -> Result<()> {
        let user_stake = &mut ctx.accounts.user_stake;
        let current_time = Clock::get()?.unix_timestamp;
        let time_since_last = current_time.saturating_sub(user_stake.last_check_in);

        // 1. THE GUILLOTINE CHECK (Closing the Loophole)
        // If they try to verify after 48 hours (172,800 seconds), block it.
        require!(time_since_last <= 172800, ErrorCode::MissedDeadline);

        // 2. THE COOLDOWN CHECK (Anti-Cheat)
        // Only enforce the 12-hour rest IF they have already completed Day 1.
        if user_stake.days_completed > 0 {
            require!(time_since_last >= 43200, ErrorCode::WorkoutTooSoon);
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

    // 3. Finish the sprint and unlock the vault
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

    // 4. THE VAMPIRE (10% Bleed for a missed day)
    pub fn slash_missed_day(ctx: Context<SlashUser>) -> Result<()> {
        // --- THE SECURITY LOCK ---
        // Ensure the SOL is routed to the official ForgeFi Treasury ONLY.
        // Replace this string with your actual Phantom Wallet public key before deploying!
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

        // THE GUILLOTINE: 48 hours = 172,800 seconds.
        require!(time_since_last > 172800, ErrorCode::DeadlineNotPassed);

        // Calculate exactly 10% of their initial locked amount
        let penalty_amount = user_stake.stake_amount / 10;

        // Bleed the PDA vault and route to the protocol treasury
        **user_stake.to_account_info().try_borrow_mut_lamports()? -= penalty_amount;
        **ctx
            .accounts
            .treasury
            .to_account_info()
            .try_borrow_mut_lamports()? += penalty_amount;

        // Record the failure and reset the 48-hour clock for their next attempt
        user_stake.missed_days = user_stake.missed_days.saturating_add(1);
        user_stake.last_check_in = current_time;

        msg!(
            "SLASHED: Lifter missed a day. 10% of initial stake ({} lamports) bled to the Graveyard.",
            penalty_amount
        );
        Ok(())
    }
}

// --- DATABASE SCHEMA ---
#[account]
pub struct UserStake {
    pub user: Pubkey,
    pub stake_amount: u64,
    pub days_committed: u8,
    pub days_completed: u8,
    pub missed_days: u8,
    pub last_check_in: i64,
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
        space = 8 + 32 + 8 + 1 + 1 + 1 + 8 + 1, 
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

    /// CHECK: The protocol's Graveyard wallet where slashed SOL is deposited
    #[account(mut)]
    pub treasury: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"stake", user_stake.user.as_ref()], 
        bump = user_stake.bump
    )]
    pub user_stake: Account<'info, UserStake>,
}

// --- CUSTOM ERROR CODES ---
#[error_code]
pub enum ErrorCode {
    #[msg("Muscles need rest. You must wait at least 12 hours between verified workouts.")]
    WorkoutTooSoon,
    #[msg("Protocol not complete. You cannot withdraw your stake until all committed days are processed.")]
    ProtocolNotComplete,
    #[msg(
        "The guillotine has not dropped yet. The lifter still has time in their 48-hour window."
    )]
    DeadlineNotPassed,
    #[msg("You missed your 48-hour window. Your stake is bleeding and awaiting liquidation.")]
    MissedDeadline,
    #[msg("Security Alert: Slashed funds can only be routed to the official ForgeFi Treasury.")]
    UnauthorizedTreasury,
}
