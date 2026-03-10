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

        // FIX: Cek deadline (Critical Issue #1)
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
        
        // FIX: Array Mismatch (Critical Issue #2)
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

// ... (Structs CreateCampaign, Contribute, Withdraw, Refund, Campaign, Contributor tetap sama)
// Pastikan di Contribute & Refund menggunakan seeds = [b"contributor".as_ref(), ...]