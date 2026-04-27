use anchor_lang::prelude::*;
use anchor_lang::system_program;

// 🚨 NEW: Import the SPL Token modules
// use anchor_spl::associated_token::AssociatedToken;
// use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

// Playground will auto-update this on build
declare_id!("AyN3aAx2VJTSxJGaR5n9Ayhpa6inCAxaSGupxbGw1Rnz");

#[program]
pub mod forgefi {
    use super::*;

    pub fn initialize_routine(
        ctx: Context<InitializeRoutine>,
        stake_amount: u64,
        days_committed: u16,
    ) -> Result<()> {
        let user_stake = &mut ctx.accounts.user_stake;
        user_stake.user = *ctx.accounts.user.key;
        user_stake.stake_amount = stake_amount;
        user_stake.days_committed = days_committed;
        user_stake.days_completed = 0;
        user_stake.missed_days = 0;
        user_stake.last_check_in = Clock::get()?.unix_timestamp;

        // Default to unused when they create the pact
        user_stake.tactical_rest_used = false;

        user_stake.bump = ctx.bumps.user_stake;

        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.user_stake.to_account_info(),
            },
        );
        system_program::transfer(cpi_context, stake_amount)?;
        Ok(())
    }

    pub fn verify_workout(ctx: Context<VerifyWorkout>) -> Result<()> {
        let user_stake = &mut ctx.accounts.user_stake;
        let current_time = Clock::get()?.unix_timestamp;

        // Stop them from verifying if the protocol is already over
        require!(
            user_stake.days_completed + user_stake.missed_days < user_stake.days_committed,
            ErrorCode::ProtocolComplete
        );

        let time_since_last = current_time.saturating_sub(user_stake.last_check_in);

        require!(time_since_last <= 60, ErrorCode::MissedDeadline);
        if user_stake.days_completed > 0 {
            require!(time_since_last >= 10, ErrorCode::WorkoutTooSoon);
        }

        user_stake.days_completed = user_stake.days_completed.saturating_add(1);
        user_stake.last_check_in = current_time;
        Ok(())
    }

    pub fn resolve_stake(ctx: Context<ResolveStake>) -> Result<()> {
        let user_stake = &mut ctx.accounts.user_stake;
        let total_days_processed = user_stake.days_completed + user_stake.missed_days;
        require!(
            total_days_processed >= user_stake.days_committed,
            ErrorCode::ProtocolNotComplete
        );
        Ok(())
    }

    pub fn slash_missed_day(ctx: Context<SlashUser>) -> Result<()> {
        let official_treasury: Pubkey = "HrAkqgXZA1fkwoJ6tdDcsu84R67yR7KCpB8NUR6oZ5NC"
            .parse()
            .unwrap();
        require!(
            ctx.accounts.treasury.key() == official_treasury,
            ErrorCode::UnauthorizedTreasury
        );

        let user_stake = &mut ctx.accounts.user_stake;
        let current_time = Clock::get()?.unix_timestamp;

        // Stop the executioner from slashing if the protocol is already over
        require!(
            user_stake.days_completed + user_stake.missed_days < user_stake.days_committed,
            ErrorCode::ProtocolComplete
        );

        let time_since_last = current_time.saturating_sub(user_stake.last_check_in);

        require!(time_since_last > 60, ErrorCode::DeadlineNotPassed);

        let penalty_amount = user_stake.stake_amount / 10;
        let current_balance = user_stake.to_account_info().lamports();
        let rent_minimum = Rent::get()?.minimum_balance(user_stake.to_account_info().data_len());
        let available_to_slash = current_balance.saturating_sub(rent_minimum);

        if available_to_slash < penalty_amount {
            user_stake
                .to_account_info()
                .sub_lamports(available_to_slash)?;
            ctx.accounts
                .treasury
                .to_account_info()
                .add_lamports(available_to_slash)?;
            user_stake.missed_days = 999;
            user_stake.last_check_in = current_time;
            return Ok(());
        }

        user_stake.to_account_info().sub_lamports(penalty_amount)?;
        ctx.accounts
            .treasury
            .to_account_info()
            .add_lamports(penalty_amount)?;
        user_stake.missed_days = user_stake.missed_days.saturating_add(1);
        user_stake.last_check_in = current_time;
        Ok(())
    }

    pub fn acknowledge_failure(_ctx: Context<AcknowledgeFailure>) -> Result<()> {
        Ok(())
    }

    pub fn use_tactical_rest(ctx: Context<UseTacticalRest>) -> Result<()> {
        let user_stake = &mut ctx.accounts.user_stake;

        // 1. You cannot rest if the protocol is already over
        require!(
            user_stake.days_completed + user_stake.missed_days < user_stake.days_committed,
            ErrorCode::ProtocolComplete
        );

        // 2. You only get one lifeline per protocol
        require!(!user_stake.tactical_rest_used, ErrorCode::RestAlreadyUsed);

        // 3. Make them pay the Blood Price (0.01 SOL)
        let fee = 10_000_000; // 0.01 SOL in lamports
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.treasury.to_account_info(),
            },
        );
        system_program::transfer(cpi_context, fee)?;

        // 4. Grant the rest: Push the guillotine back by 60 Devnet seconds (1 Day)
        user_stake.tactical_rest_used = true;
        user_stake.last_check_in += 60;

        Ok(())
    }

    pub fn initialize_squad(
        ctx: Context<InitializeSquad>,
        required_stake: u64,
        days: u16,
        player_two: Pubkey,
        player_three: Pubkey,
    ) -> Result<()> {
        let vault = &mut ctx.accounts.squad_vault;
        let player_one = &ctx.accounts.player_one;

        vault.player_one = player_one.key();
        vault.player_two = player_two;
        vault.player_three = player_three;
        vault.required_stake_per_player = required_stake;
        vault.days_committed = days;

        vault.days_completed = 0;
        vault.missed_days = 0;
        vault.bump = ctx.bumps.squad_vault;

        vault.p1_workouts = 0;
        vault.p2_workouts = 0;
        vault.p3_workouts = 0;

        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: player_one.to_account_info(),
                to: vault.to_account_info(),
            },
        );
        system_program::transfer(cpi_context, required_stake)?;

        vault.total_vault_balance = required_stake;
        vault.p1_staked = true;
        vault.p2_staked = false;
        vault.p3_staked = false;
        vault.protocol_active = false;
        Ok(())
    }

    pub fn join_squad(ctx: Context<JoinSquad>) -> Result<()> {
        let vault = &mut ctx.accounts.squad_vault;
        let joining_player = &ctx.accounts.player;

        require!(
            joining_player.key() == vault.player_two || joining_player.key() == vault.player_three,
            ErrorCode::NotInvited
        );

        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: joining_player.to_account_info(),
                to: vault.to_account_info(),
            },
        );
        system_program::transfer(cpi_context, vault.required_stake_per_player)?;

        if joining_player.key() == vault.player_two {
            vault.p2_staked = true;
        } else if joining_player.key() == vault.player_three {
            vault.p3_staked = true;
        }

        vault.total_vault_balance += vault.required_stake_per_player;

        let is_duo = vault.player_three == Pubkey::default();
        if (is_duo && vault.p1_staked && vault.p2_staked)
            || (!is_duo && vault.p1_staked && vault.p2_staked && vault.p3_staked)
        {
            vault.protocol_active = true;
            let current_time = Clock::get()?.unix_timestamp;
            vault.p1_last_check_in = current_time;
            vault.p2_last_check_in = current_time;
            vault.p3_last_check_in = current_time;
        }
        Ok(())
    }

    pub fn verify_squad_workout(ctx: Context<VerifySquadWorkout>) -> Result<()> {
        let vault = &mut ctx.accounts.squad_vault;
        let player = &ctx.accounts.player;
        let current_time = Clock::get()?.unix_timestamp;

        require!(vault.protocol_active, ErrorCode::ProtocolNotActive);

        // 🚨 SQUAD CAP: Stop them from verifying if the squad protocol is already over
        require!(
            vault.days_completed + vault.missed_days < vault.days_committed,
            ErrorCode::ProtocolComplete
        );

        // 🚨 COMPILER FIX: Removed mut and = 0
        let time_since_last: i64;
        let mut is_p1 = false;
        let mut is_p2 = false;
        let mut is_p3 = false;

        if player.key() == vault.player_one {
            time_since_last = current_time.saturating_sub(vault.p1_last_check_in);
            is_p1 = true;
        } else if player.key() == vault.player_two {
            time_since_last = current_time.saturating_sub(vault.p2_last_check_in);
            is_p2 = true;
        } else if player.key() == vault.player_three {
            time_since_last = current_time.saturating_sub(vault.p3_last_check_in);
            is_p3 = true;
        } else {
            return err!(ErrorCode::NotInvited);
        }

        require!(time_since_last <= 60, ErrorCode::MissedDeadline);

        let player_workouts = if is_p1 {
            vault.p1_workouts
        } else if is_p2 {
            vault.p2_workouts
        } else {
            vault.p3_workouts
        };

        if player_workouts > 0 {
            require!(time_since_last >= 10, ErrorCode::WorkoutTooSoon);
        }

        if is_p1 {
            vault.p1_last_check_in = current_time;
            vault.p1_workouts = vault.p1_workouts.saturating_add(1);
        } else if is_p2 {
            vault.p2_last_check_in = current_time;
            vault.p2_workouts = vault.p2_workouts.saturating_add(1);
        } else if is_p3 {
            vault.p3_last_check_in = current_time;
            vault.p3_workouts = vault.p3_workouts.saturating_add(1);
        }

        let is_duo = vault.player_three == Pubkey::default();
        let min_workouts = if is_duo {
            vault.p1_workouts.min(vault.p2_workouts)
        } else {
            vault
                .p1_workouts
                .min(vault.p2_workouts)
                .min(vault.p3_workouts)
        };

        if min_workouts > vault.days_completed {
            vault.days_completed = min_workouts;
        }
        Ok(())
    }

    pub fn slash_squad(ctx: Context<SlashSquad>) -> Result<()> {
        let official_treasury: Pubkey = "HrAkqgXZA1fkwoJ6tdDcsu84R67yR7KCpB8NUR6oZ5NC"
            .parse()
            .unwrap();
        require!(
            ctx.accounts.treasury.key() == official_treasury,
            ErrorCode::UnauthorizedTreasury
        );

        let vault = &mut ctx.accounts.squad_vault;
        let current_time = Clock::get()?.unix_timestamp;

        // 🚨 SQUAD CAP: Stop the executioner from slashing if the squad protocol is already over
        require!(
            vault.days_completed + vault.missed_days < vault.days_committed,
            ErrorCode::ProtocolComplete
        );

        let p1_time = current_time.saturating_sub(vault.p1_last_check_in);
        let p2_time = current_time.saturating_sub(vault.p2_last_check_in);
        let mut p3_time = 0;

        if vault.player_three != Pubkey::default() {
            p3_time = current_time.saturating_sub(vault.p3_last_check_in);
        }

        require!(
            p1_time > 60
                || p2_time > 60
                || (vault.player_three != Pubkey::default() && p3_time > 60),
            ErrorCode::DeadlineNotPassed
        );

        let penalty_amount = vault.total_vault_balance / 10;
        let current_balance = vault.to_account_info().lamports();
        let rent_minimum = Rent::get()?.minimum_balance(vault.to_account_info().data_len());
        let available_to_slash = current_balance.saturating_sub(rent_minimum);

        if available_to_slash < penalty_amount {
            vault.to_account_info().sub_lamports(available_to_slash)?;
            ctx.accounts
                .treasury
                .to_account_info()
                .add_lamports(available_to_slash)?;
            vault.missed_days = 999;
            vault.p1_last_check_in = current_time;
            vault.p2_last_check_in = current_time;
            vault.p3_last_check_in = current_time;
            return Ok(());
        }

        vault.to_account_info().sub_lamports(penalty_amount)?;
        ctx.accounts
            .treasury
            .to_account_info()
            .add_lamports(penalty_amount)?;
        vault.missed_days = vault.missed_days.saturating_add(1);

        vault.p1_last_check_in = current_time;
        vault.p2_last_check_in = current_time;
        if vault.player_three != Pubkey::default() {
            vault.p3_last_check_in = current_time;
        }

        Ok(())
    }

    pub fn acknowledge_squad_failure(_ctx: Context<AcknowledgeSquadFailure>) -> Result<()> {
        Ok(())
    }

    // 🚨 NEW: Claim Victory and Burn Squad Vault 🚨
    // 🚨 UPDATED: Mathematically divide the surviving SOL before closing 🚨
    // 🚨 UPDATED: Added Security Check for P3 🚨
    pub fn resolve_squad_stake(ctx: Context<ResolveSquadStake>) -> Result<()> {
        let vault = &mut ctx.accounts.squad_vault;

        let mut player_count = 2;
        if vault.player_three != Pubkey::default() {
            player_count = 3;
        }

        let current_balance = vault.to_account_info().lamports();
        let rent_minimum = Rent::get()?.minimum_balance(vault.to_account_info().data_len());
        let available_to_distribute = current_balance.saturating_sub(rent_minimum);

        let share = available_to_distribute / player_count;

        vault.to_account_info().sub_lamports(share)?;
        ctx.accounts.player_one.add_lamports(share)?;

        vault.to_account_info().sub_lamports(share)?;
        ctx.accounts.player_two.add_lamports(share)?;

        // Distribute to Player 3 (if applicable)
        if player_count == 3 {
            // SECURITY CHECK: Ensure the provided account actually belongs to Player 3
            require!(
                ctx.accounts.player_three.key() == vault.player_three,
                ErrorCode::NotInvited
            );

            vault.to_account_info().sub_lamports(share)?;
            ctx.accounts.player_three.add_lamports(share)?;
        }

        Ok(())
    }
}

#[account]
pub struct UserStake {
    pub user: Pubkey,
    //pub mint: Pubkey, // 🚨 NEW: Tracks which LST (e.g. JitoSOL) is locked here
    pub stake_amount: u64, // Now represents Token balance, not Lamports
    pub days_committed: u16,
    pub days_completed: u16,
    pub missed_days: u16,
    pub last_check_in: i64,
    pub tactical_rest_used: bool, // 🚨 NEW: Tracks the one-time lifeline
    pub bump: u8,
}

#[account]
pub struct SquadVaultV2 {
    pub player_one: Pubkey,
    pub player_two: Pubkey,
    pub player_three: Pubkey,
    //pub mint: Pubkey,  // 🚨 NEW: Tracks the LST for the squad
    pub required_stake_per_player: u64,
    pub total_vault_balance: u64,
    pub days_committed: u16,
    pub days_completed: u16,
    pub missed_days: u16,

    pub p1_workouts: u16,
    pub p2_workouts: u16,
    pub p3_workouts: u16,

    pub p1_staked: bool,
    pub p2_staked: bool,
    pub p3_staked: bool,
    pub protocol_active: bool,
    pub p1_last_check_in: i64,
    pub p2_last_check_in: i64,
    pub p3_last_check_in: i64,
    pub bump: u8,
}

#[derive(Accounts)]
pub struct InitializeRoutine<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(init, payer = user, space = 8 + 32 + 8 + 2 + 2 + 2 + 8 + 1 + 1, seeds = [b"stake_v2", user.key().as_ref()], bump)]
    pub user_stake: Account<'info, UserStake>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct VerifyWorkout<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut, seeds = [b"stake_v2", user.key().as_ref()], bump = user_stake.bump)]
    pub user_stake: Account<'info, UserStake>,
}

#[derive(Accounts)]
pub struct ResolveStake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut, close = user, seeds = [b"stake_v2", user.key().as_ref()], bump = user_stake.bump, constraint = user_stake.days_completed + user_stake.missed_days >= user_stake.days_committed @ ErrorCode::ProtocolNotComplete)]
    pub user_stake: Account<'info, UserStake>,
}

#[derive(Accounts)]
pub struct SlashUser<'info> {
    #[account(mut)]
    pub liquidator: Signer<'info>,
    #[account(mut)]
    pub treasury: AccountInfo<'info>,
    #[account(mut, seeds = [b"stake_v2", user_stake.user.as_ref()], bump = user_stake.bump)]
    pub user_stake: Account<'info, UserStake>,
}

#[derive(Accounts)]
pub struct AcknowledgeFailure<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut, close = user, seeds = [b"stake_v2", user.key().as_ref()], bump = user_stake.bump, constraint = user_stake.missed_days == 999 @ ErrorCode::NotAZombie)]
    pub user_stake: Account<'info, UserStake>,
}

#[derive(Accounts)]
pub struct UseTacticalRest<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, seeds = [b"stake_v2", user.key().as_ref()], bump = user_stake.bump)]
    pub user_stake: Account<'info, UserStake>,

    /// CHECK: The official ForgeFi Treasury
    #[account(mut, address = "HrAkqgXZA1fkwoJ6tdDcsu84R67yR7KCpB8NUR6oZ5NC".parse::<Pubkey>().unwrap())]
    pub treasury: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(required_stake: u64, days: u16, player_two: Pubkey, player_three: Pubkey)]
pub struct InitializeSquad<'info> {
    #[account(mut)]
    pub player_one: Signer<'info>,
    #[account(init, payer = player_one, space = 8 + 200, seeds = [b"squad_v2", player_one.key().as_ref(), player_two.as_ref()], bump)]
    pub squad_vault: Account<'info, SquadVaultV2>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct JoinSquad<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(mut, seeds = [b"squad_v2", squad_vault.player_one.as_ref(), squad_vault.player_two.as_ref()], bump = squad_vault.bump)]
    pub squad_vault: Account<'info, SquadVaultV2>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct VerifySquadWorkout<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(mut, seeds = [b"squad_v2", squad_vault.player_one.as_ref(), squad_vault.player_two.as_ref()], bump = squad_vault.bump)]
    pub squad_vault: Account<'info, SquadVaultV2>,
}

#[derive(Accounts)]
pub struct SlashSquad<'info> {
    #[account(mut)]
    pub liquidator: Signer<'info>,
    #[account(mut)]
    pub treasury: AccountInfo<'info>,
    #[account(mut, seeds = [b"squad_v2", squad_vault.player_one.as_ref(), squad_vault.player_two.as_ref()], bump = squad_vault.bump)]
    pub squad_vault: Account<'info, SquadVaultV2>,
}

#[derive(Accounts)]
pub struct AcknowledgeSquadFailure<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        mut, 
        close = player, 
        seeds = [b"squad_v2", squad_vault.player_one.as_ref(), squad_vault.player_two.as_ref()], 
        bump = squad_vault.bump, 
        constraint = squad_vault.missed_days == 999 @ ErrorCode::NotAZombie
    )]
    pub squad_vault: Account<'info, SquadVaultV2>,
}

// 🚨 NEW: The Squad Resolution Struct 🚨
// 🚨 UPDATED: We need to pull the actual player accounts into the struct to pay them 🚨
#[derive(Accounts)]
pub struct ResolveSquadStake<'info> {
    #[account(mut)]
    pub player: Signer<'info>, // Whoever pays the gas to trigger the resolution

    /// CHECK: We are just sending them their share of the SOL
    #[account(mut, address = squad_vault.player_one)]
    pub player_one: AccountInfo<'info>,

    /// CHECK: We are just sending them their share of the SOL
    #[account(mut, address = squad_vault.player_two)]
    pub player_two: AccountInfo<'info>,

    /// CHECK: We are just sending them their share of the SOL (Defaults to System Program if empty)
    #[account(mut)]
    pub player_three: AccountInfo<'info>,

    #[account(
        mut, 
        close = player, 
        seeds = [b"squad_v2", squad_vault.player_one.as_ref(), squad_vault.player_two.as_ref()], 
        bump = squad_vault.bump, 
        constraint = squad_vault.days_completed + squad_vault.missed_days >= squad_vault.days_committed @ ErrorCode::ProtocolNotComplete
    )]
    pub squad_vault: Account<'info, SquadVaultV2>,
}

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
    #[msg("The Blood Pact is not active yet. Waiting for all players to join.")]
    ProtocolNotActive,
    #[msg("The protocol has concluded. Please resolve your stake to withdraw remaining funds.")]
    ProtocolComplete,
    #[msg("You have already used your Tactical Rest. No mercy.")]
    RestAlreadyUsed,
}
