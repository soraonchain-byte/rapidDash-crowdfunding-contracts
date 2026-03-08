use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction;
use anchor_lang::solana_program::program::invoke_signed;

declare_id!("8vS5U7fEaFmYt1GvK9P2XwQ7R6L4H3J2M1N0B9V8C7X6");

#[program]
pub mod crowdfunding {
    use super::*;

    pub fn create_campaign(ctx: Context<CreateCampaign>, goal: u64, deadline: i64) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        let clock = Clock::get()?;
        
        // Validate deadline is in the future
        require!(deadline > clock.unix_timestamp, ErrorCode::InvalidDeadline);

        campaign.creator = ctx.accounts.creator.key();
        campaign.goal = goal;
        campaign.deadline = deadline;
        campaign.raised = 0;
        campaign.claimed = false;

        msg!("Campaign created: goal={}, deadline={}", goal, deadline);
        Ok(())
    }

    pub fn contribute(ctx: Context<Contribute>, amount: u64) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        
        // Transfer SOL from donor to PDA Vault
        let ix = system_instruction::transfer(
            &ctx.accounts.user.key(),
            &ctx.accounts.vault.key(),
            amount,
        );
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.vault.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        campaign.raised += amount;
        msg!("Contributed: {} lamports, total={}", amount, campaign.raised);
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        let clock = Clock::get()?;

        // Conditions: raised >= goal, current_time >= deadline, caller is creator, not claimed
        require!(campaign.raised >= campaign.goal, ErrorCode::GoalNotReached);
        require!(clock.unix_timestamp >= campaign.deadline, ErrorCode::CampaignNotEnded);
        require!(!campaign.claimed, ErrorCode::AlreadyClaimed);

        let amount = ctx.accounts.vault.lamports();
        let campaign_key = campaign.key();
        let seeds = &[b"vault", campaign_key.as_ref(), &[ctx.bumps.vault]];
        let signer = &[&seeds[..]];

        // Transfer from Vault to Creator using invoke_signed
        invoke_signed(
            &system_instruction::transfer(&ctx.accounts.vault.key(), &ctx.accounts.creator.key(), amount),
            &[
                ctx.accounts.vault.to_account_info(),
                ctx.accounts.creator.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            signer,
        )?;

        campaign.claimed = true;
        msg!("Withdrawn: {} lamports", amount);
        Ok(())
    }

    pub fn refund(ctx: Context<Refund>, amount: u64) -> Result<()> {
        let campaign = &ctx.accounts.campaign;
        let clock = Clock::get()?;

        // Conditions: raised < goal, current_time >= deadline
        require!(campaign.raised < campaign.goal, ErrorCode::GoalReached);
        require!(clock.unix_timestamp >= campaign.deadline, ErrorCode::CampaignNotEnded);

        let campaign_key = campaign.key();
        let seeds = &[b"vault", campaign_key.as_ref(), &[ctx.bumps.vault]];
        let signer = &[&seeds[..]];

        invoke_signed(
            &system_instruction::transfer(&ctx.accounts.vault.key(), &ctx.accounts.user.key(), amount),
            &[
                ctx.accounts.vault.to_account_info(),
                ctx.accounts.user.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            signer,
        )?;

        msg!("Refunded: {} lamports", amount);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateCampaign<'info> {
    #[account(init, payer = creator, space = 8 + 32 + 8 + 8 + 8 + 1)]
    pub campaign: Account<'info, Campaign>,
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Contribute<'info> {
    #[account(mut)]
    pub campaign: Account<'info, Campaign>,
    /// CHECK: PDA Vault
    #[account(mut, seeds = [b"vault", campaign.key().as_ref()], bump)]
    pub vault: AccountInfo<'info>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut, has_one = creator)]
    pub campaign: Account<'info, Campaign>,
    #[account(mut, seeds = [b"vault", campaign.key().as_ref()], bump)]
    pub vault: AccountInfo<'info>,
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Refund<'info> {
    #[account(mut)]
    pub campaign: Account<'info, Campaign>,
    #[account(mut, seeds = [b"vault", campaign.key().as_ref()], bump)]
    pub vault: AccountInfo<'info>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct Campaign {
    pub creator: Pubkey,
    pub goal: u64,
    pub raised: u64,
    pub deadline: i64,
    pub claimed: bool,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Deadline must be in the future")]
    InvalidDeadline,
    #[msg("Campaign goal not reached")]
    GoalNotReached,
    #[msg("Campaign goal reached, no refunds")]
    GoalReached,
    #[msg("Campaign is still active")]
    CampaignNotEnded,
    #[msg("Funds already claimed")]
    AlreadyClaimed,
}