use anchor_lang::prelude::*;

// Pastikan ID ini sama dengan yang ada di Anchor.toml nanti
declare_id!("8vS5U7fEaFmYt1GvK9P2XwQ7R6L4H3J2M1N0B9V8C7X6");

#[program]
pub mod crowdfunding {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, target: u64) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        campaign.author = ctx.accounts.author.key();
        campaign.target_amount = target;
        campaign.amount_raised = 0;
        msg!("Campaign Initialized for author: {} with target: {}", campaign.author, target);
        Ok(())
    }

    pub fn donate(ctx: Context<Donate>, amount: u64) -> Result<()> {
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.campaign.to_account_info(),
            },
        );
        anchor_lang::system_program::transfer(cpi_context, amount)?;
        ctx.accounts.campaign.amount_raised += amount;
        msg!("Received donation of {} lamports", amount);
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        // Sesuai panduan: Cek target sebelum dana ditarik
        require!(campaign.amount_raised >= campaign.target_amount, ErrorCode::TargetNotReached);
        
        let amount = campaign.to_account_info().lamports();
        **campaign.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.author.to_account_info().try_borrow_mut_lamports()? += amount;
        msg!("Funds withdrawn successfully");
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    // 8 discriminator + 32 pubkey + 8 u64 + 8 u64 = 56 bytes
    #[account(init, payer = author, space = 8 + 32 + 8 + 8)]
    pub campaign: Account<'info, Campaign>,
    #[account(mut)]
    pub author: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Donate<'info> {
    #[account(mut)]
    pub campaign: Account<'info, Campaign>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut, has_one = author @ ErrorCode::Unauthorized)]
    pub campaign: Account<'info, Campaign>,
    #[account(mut)]
    pub author: Signer<'info>,
}

#[account]
pub struct Campaign {
    pub author: Pubkey,
    pub target_amount: u64,
    pub amount_raised: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Target dana belum tercapai!")]
    TargetNotReached,
    #[msg("Hanya author yang diizinkan melakukan ini.")]
    Unauthorized,
}