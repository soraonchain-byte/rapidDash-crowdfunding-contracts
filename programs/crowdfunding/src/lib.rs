use anchor_lang::prelude::*;
use anchor_lang::solana_program::{system_instruction, program::{invoke, invoke_signed}};

declare_id!("8vS5U7fEaFmYt1GvK9P2XwQ7R6L4H3J2M1N0B9V8C7X6");

#[program]
pub mod crowdfunding {
    use super::*;

    pub fn create_campaign(ctx: Context<CreateCampaign>, goal: u64, deadline: i64) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        let clock = Clock::get()?;
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
        let clock = Clock::get()?;

        require!(clock.unix_timestamp < campaign.deadline, ErrorCode::CampaignEnded);

        let ix = system_instruction::transfer(&ctx.accounts.user.key(), &ctx.accounts.vault.key(), amount);
        invoke(&ix, &[
            ctx.accounts.user.to_account_info(),
            ctx.accounts.vault.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ])?;

        campaign.raised = campaign.raised.checked_add(amount).ok_or(ErrorCode::Overflow)?;
        ctx.accounts.contributor_account.amount = ctx.accounts.contributor_account.amount.checked_add(amount).ok_or(ErrorCode::Overflow)?;
        
        msg!("Contributed: {} lamports, total={}", amount, campaign.raised);
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        let clock = Clock::get()?;

        require!(campaign.raised >= campaign.goal, ErrorCode::GoalNotReached);
        require!(clock.unix_timestamp >= campaign.deadline, ErrorCode::CampaignNotEnded);
        require!(!campaign.claimed, ErrorCode::AlreadyClaimed);

        let amount = ctx.accounts.vault.lamports();
        let campaign_key = campaign.key();
        let seeds: &[&[u8]] = &[
            b"vault".as_ref(),
            campaign_key.as_ref(),
            &[ctx.bumps.vault],
        ];

        invoke_signed(
            &system_instruction::transfer(&ctx.accounts.vault.key(), &ctx.accounts.creator.key(), amount),
            &[ctx.accounts.vault.to_account_info(), ctx.accounts.creator.to_account_info(), ctx.accounts.system_program.to_account_info()],
            &[seeds],
        )?;

        campaign.claimed = true;
        msg!("Withdrawn: {} lamports", amount);
        Ok(())
    }

    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        let campaign = &ctx.accounts.campaign;
        let clock = Clock::get()?;

        require!(campaign.raised < campaign.goal, ErrorCode::GoalReached);
        require!(clock.unix_timestamp >= campaign.deadline, ErrorCode::CampaignNotEnded);
        
        let amount = ctx.accounts.contributor_account.amount;
        require!(amount > 0, ErrorCode::NoContribution);

        let campaign_key = campaign.key();
        let seeds: &[&[u8]] = &[
            b"vault".as_ref(),
            campaign_key.as_ref(),
            &[ctx.bumps.vault],
        ];

        invoke_signed(
            &system_instruction::transfer(&ctx.accounts.vault.key(), &ctx.accounts.user.key(), amount),
            &[ctx.accounts.vault.to_account_info(), ctx.accounts.user.to_account_info(), ctx.accounts.system_program.to_account_info()],
            &[seeds],
        )?;

        ctx.accounts.contributor_account.amount = 0;
        msg!("Refunded: {} lamports", amount);
        Ok(())
    }
}

// --- ACCOUNT STRUCTS (BAGIAN INI YANG TADI HILANG/TERPOTONG) ---

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
    #[account(
        init_if_needed, 
        payer = user, 
        space = 8 + 8, 
        seeds = [b"contributor".as_ref(), campaign.key().as_ref(), user.key().as_ref()], 
        bump
    )]
    pub contributor_account: Account<'info, Contributor>,
    #[account(mut, seeds = [b"vault".as_ref(), campaign.key().as_ref()], bump)]
    pub vault: SystemAccount<'info>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut, has_one = creator)] // Cek otomatis: campaign.creator == creator.key
    pub campaign: Account<'info, Campaign>,
    #[account(mut, seeds = [b"vault".as_ref(), campaign.key().as_ref()], bump)]
    pub vault: SystemAccount<'info>,
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Refund<'info> {
    pub campaign: Account<'info, Campaign>,
    #[account(
        mut, 
        seeds = [b"contributor".as_ref(), campaign.key().as_ref(), user.key().as_ref()], 
        bump
    )]
    pub contributor_account: Account<'info, Contributor>,
    #[account(mut, seeds = [b"vault".as_ref(), campaign.key().as_ref()], bump)]
    pub vault: SystemAccount<'info>,
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

#[account]
pub struct Contributor {
    pub amount: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Deadline must be in the future")]
    InvalidDeadline,
    #[msg("Campaign has ended")]
    CampaignEnded,
    #[msg("Goal not reached")]
    GoalNotReached,
    #[msg("Goal reached, no refunds")]
    GoalReached,
    #[msg("Campaign still active")]
    CampaignNotEnded,
    #[msg("Already claimed")]
    AlreadyClaimed,
    #[msg("No contribution found")]
    NoContribution,
    #[msg("Arithmetic overflow")]
    Overflow,
}