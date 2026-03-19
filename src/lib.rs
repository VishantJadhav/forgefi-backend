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

    // 2. Daily check-in (The Executioner)
    pub fn verify_workout(ctx: Context<VerifyWorkout>) -> Result<()> {
        let user_stake = &mut ctx.accounts.user_stake;
        let current_time = Clock::get()?.unix_timestamp;

        // 1. TIME LOCK: Ensure at least 12 hours (43,200 seconds) have passed.
        // NOTE FOR TESTING: Change 43200 to 60 if you want to test this quickly on Devnet (1 minute wait)
        let time_since_last = current_time.saturating_sub(user_stake.last_check_in);
        require!(time_since_last >= 43200, ErrorCode::WorkoutTooSoon);

        // 2. INCREMENT THE MATRIX
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

        // SECURITY: Ensure the lifter actually completed the protocol before allowing withdrawal
        require!(
            user_stake.days_completed >= user_stake.days_committed,
            ErrorCode::ProtocolNotComplete
        );

        let amount = user_stake.stake_amount;

        // Unlock the PDA and return the funds
        **user_stake.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx
            .accounts
            .user
            .to_account_info()
            .try_borrow_mut_lamports()? += amount;

        msg!(
            "Sprint complete! Stake of {} returned safely. Stay hard.",
            amount
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
        // Memory allocated: discriminator (8) + pubkey (32) + u64 (8) + u8 (1) + u8 (1) + i64 (8) + u8 (1)
        space = 8 + 32 + 8 + 1 + 1 + 8 + 1, 
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
    // The "close" tag automatically securely empties the vault and refunds storage rent!
    #[account(
        mut,
        close = user, 
        seeds = [b"stake", user.key().as_ref()], 
        bump = user_stake.bump
    )]
    pub user_stake: Account<'info, UserStake>,
}

// --- CUSTOM ERROR CODES ---
#[error_code]
pub enum ErrorCode {
    #[msg("Muscles need rest. You must wait at least 12 hours between verified workouts.")]
    WorkoutTooSoon,
    #[msg(
        "Protocol not complete. You cannot withdraw your stake until the Iron Matrix is filled."
    )]
    ProtocolNotComplete,
}
