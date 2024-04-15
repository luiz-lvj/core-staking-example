use anchor_lang::prelude::*;
use mpl_core::{
    instructions::{
        AddPluginV1Cpi, AddPluginV1CpiAccounts, AddPluginV1InstructionArgs, 
        UpdatePluginV1Cpi, UpdatePluginV1CpiAccounts, UpdatePluginV1InstructionArgs,
        RemovePluginV1Cpi, RemovePluginV1CpiAccounts, RemovePluginV1InstructionArgs,
    },
    ID as core_program_id,
    types::{FreezeDelegate, Plugin, PluginType, PluginAuthority}
};

declare_id!("J4sLf325UH7YrSUXsNsQs5xCptmS5WuTCYAk6g52phpp");

#[program]
pub mod core_staking_example {
    use super::*;

    pub fn create_staking_account(ctx: Context<CreateStakingAccount>) -> Result<()> {
        let staking_account = &mut ctx.accounts.staking_account;
        staking_account.set_inner(
            StakingAccount {
                owner: *ctx.accounts.owner.key,
                staked: 0,
                time_staked: 0,
                bump: ctx.bumps.staking_account,
            }
        );

        Ok(())
    }

    pub fn stake(ctx: Context<Stake>) -> Result<()> {        
        // To freeze the asset, we should use the `FreezeDelegate` Owner-managed Plugin. 
        // In this occasion the NFT has a collection so we included that in the instruction 
        // but that's an optional account.
        // The authorithy over the freeze will be the init_authority that can be None, the owner,
        // the update authority or just an address [Check PluginAuthority Types]
        AddPluginV1Cpi::new(
            ctx.accounts.core_program.to_account_info().as_ref(),
            AddPluginV1CpiAccounts {
                asset: ctx.accounts.asset.as_ref(),
                collection: Some(&ctx.accounts.collection),
                payer: &ctx.accounts.owner.to_account_info(),
                authority: Some(&ctx.accounts.owner.to_account_info()),
                system_program: &ctx.accounts.system_program.to_account_info(),
                log_wrapper: None,
            },
            AddPluginV1InstructionArgs {
                plugin: Plugin::FreezeDelegate(FreezeDelegate {frozen: true}),
                init_authority: Some(PluginAuthority::Address {address: ctx.accounts.staking_account.key()})
            }
        ).invoke()?;

        let staking_account = &mut ctx.accounts.staking_account;
        staking_account.staked = Clock::get()?.unix_timestamp;

        Ok(())
    }

    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        let staking_account = &mut ctx.accounts.staking_account;
        let bump = &[staking_account.bump];
        staking_account.time_staked = staking_account.time_staked.checked_add(Clock::get()?.unix_timestamp.checked_sub(staking_account.staked).ok_or(StakingError::Underflow)?).ok_or(StakingError::Overflow)?;
        staking_account.staked = 0;

        // To remove the plugin (and so the possibility of freezing the asset again) we
        // need to unfreeze the asset before (there is a check for this in the RemovePlugin
        // instruction). So we just update the FreezeDelegate Plugin: frozen: false.
        // Note: the authorithy over this action is the FreezeAuthority
        let signer_seeds = &[&[b"staking_account", ctx.accounts.owner.to_account_info().key.as_ref(), bump][..]];        
        UpdatePluginV1Cpi::new(
            ctx.accounts.core_program.to_account_info().as_ref(),
            UpdatePluginV1CpiAccounts {
                asset: ctx.accounts.asset.as_ref(),
                collection: Some(&ctx.accounts.collection),
                payer: &ctx.accounts.owner.to_account_info(),
                authority: Some(&ctx.accounts.staking_account.to_account_info()),
                system_program: &ctx.accounts.system_program.to_account_info(),
                log_wrapper: None,
            },
            UpdatePluginV1InstructionArgs {
                plugin: Plugin::FreezeDelegate(FreezeDelegate {frozen: false}),
            }
        ).invoke_signed(signer_seeds)?;

        // Now that we unfrozen the Asset we can proceed to remove the FreezeDelegate Plugin
        RemovePluginV1Cpi::new(
            ctx.accounts.core_program.to_account_info().as_ref(),
            RemovePluginV1CpiAccounts {
                asset: ctx.accounts.asset.as_ref(),
                collection: Some(&ctx.accounts.collection),
                payer: &ctx.accounts.owner.to_account_info(),
                authority: Some(&ctx.accounts.owner.to_account_info()),
                system_program: &ctx.accounts.system_program.to_account_info(),
                log_wrapper: None,
            },
            RemovePluginV1InstructionArgs {
                plugin_type: PluginType::FreezeDelegate,
            }
        ).invoke()?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateStakingAccount<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        init,
        payer = owner,
        space = StakingAccount::INIT_SPACE,
        seeds = [b"staking_account", owner.key().as_ref()],
        bump
    )]
    pub staking_account: Account<'info, StakingAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut)]
    /// CHECK: this will be checked by core
    pub asset: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: this will be checked by core
    pub collection: UncheckedAccount<'info>,
    #[account(
        seeds = [b"staking_account", owner.key().as_ref()],
        bump = staking_account.bump
    )]
    pub staking_account: Account<'info, StakingAccount>,
    #[account(address = core_program_id)]
    /// CHECK: this will be checked by core
    pub core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut)]
    /// CHECK: this will be checked by core
    pub asset: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: this will be checked by core
    pub collection: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"staking_account", owner.key().as_ref()],
        bump = staking_account.bump
    )]
    pub staking_account: Account<'info, StakingAccount>,
    #[account(address = core_program_id)]
    /// CHECK: this will be checked by core
    pub core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct StakingAccount {
    pub owner: Pubkey,
    pub staked: i64,
    pub time_staked: i64,
    pub bump: u8,
}

impl Space for StakingAccount {
    const INIT_SPACE: usize = 8 + 32 + 8 + 8 + 1;
}

#[error_code]
pub enum StakingError {
    #[msg("Invalid owner")]
    OwnerMismatched,
    #[msg("Underflow")]
    Underflow,
    #[msg("Overflow")]
    Overflow,
}