use anchor_lang::prelude::*;
use mpl_core::{
    ID as core_program_id,
    Asset,
    instructions::{
        AddPluginV1Cpi, AddPluginV1CpiAccounts, AddPluginV1InstructionArgs, 
        UpdatePluginV1Cpi, UpdatePluginV1CpiAccounts, UpdatePluginV1InstructionArgs,
        RemovePluginV1Cpi, RemovePluginV1CpiAccounts, RemovePluginV1InstructionArgs,
    },
    types::{FreezeDelegate, Plugin, PluginType, PluginAuthority, Attribute, Attributes}
};

declare_id!("J4sLf325UH7YrSUXsNsQs5xCptmS5WuTCYAk6g52phpp");

#[program]
pub mod core_staking_example {
    use super::*; 

    pub fn stake(ctx: Context<Stake>) -> Result<()> {      
        // Freeze the asset  
        AddPluginV1Cpi::new(
            ctx.accounts.core_program.to_account_info().as_ref(),
            AddPluginV1CpiAccounts {
                asset: ctx.accounts.asset.as_ref(),
                collection: Some(&ctx.accounts.collection),
                payer: &ctx.accounts.payer.to_account_info(),
                authority: Some(&ctx.accounts.signer.to_account_info()),
                system_program: &ctx.accounts.system_program.to_account_info(),
                log_wrapper: None,
            },
            AddPluginV1InstructionArgs {
                plugin: Plugin::FreezeDelegate( FreezeDelegate{ frozen: true } ),
                init_authority: Some(PluginAuthority::UpdateAuthority)
            }
        ).invoke()?;

        
        // Save data on the attributes of the Core Asset
        let info = ctx.accounts.asset.to_account_info();
        let data = info.try_borrow_mut_data()?;
        let asset = Asset::deserialize(&data)?;
        
        if asset.plugin_list.attributes.is_none() {
            AddPluginV1Cpi::new(
                ctx.accounts.core_program.to_account_info().as_ref(),
                AddPluginV1CpiAccounts {
                    asset: ctx.accounts.asset.as_ref(),
                    collection: Some(&ctx.accounts.collection),
                    payer: &ctx.accounts.payer.to_account_info(),
                    authority: Some(&ctx.accounts.update_authority.to_account_info()),
                    system_program: &ctx.accounts.system_program.to_account_info(),
                    log_wrapper: None,
                },
                AddPluginV1InstructionArgs {
                    plugin: Plugin::Attributes(
                        Attributes{ attribute_list: vec![
                            Attribute { key: "staked".to_string(), value: Clock::get()?.unix_timestamp.to_string() },
                            Attribute { key: "staked_time".to_string(), value: 0.to_string() },
                        ] }
                    ),
                    init_authority: Some(PluginAuthority::UpdateAuthority)
                }
            ).invoke()?;
        } else {
            let mut attribute_list: Vec<Attribute> = Vec::new();
            let mut is_initialized: bool = false;

            for attribute in asset.plugin_list.attributes.unwrap().attributes.attribute_list {
                if attribute.key == "staked" {
                    require!(attribute.value == "0", StakingError::AlreadyStaked);
                    attribute_list.push(Attribute { key: "staked".to_string(), value: Clock::get()?.unix_timestamp.to_string() });
                    is_initialized = true;
                } else {
                    attribute_list.push(attribute);
                } 
            }

            if is_initialized == false {
                attribute_list.push(Attribute { key: "staked".to_string(), value: Clock::get()?.unix_timestamp.to_string() });
                attribute_list.push(Attribute { key: "staked_time".to_string(), value: 0.to_string() });
            }

            UpdatePluginV1Cpi::new(
                ctx.accounts.core_program.to_account_info().as_ref(),
                UpdatePluginV1CpiAccounts {
                    asset: ctx.accounts.asset.as_ref(),
                    collection: Some(&ctx.accounts.collection),
                    payer: &ctx.accounts.payer.to_account_info(),
                    authority: Some(&ctx.accounts.update_authority.to_account_info()),
                    system_program: &ctx.accounts.system_program.to_account_info(),
                    log_wrapper: None,
                },
                UpdatePluginV1InstructionArgs {
                    plugin: Plugin::Attributes(Attributes{ attribute_list }),
                }
            ).invoke()?;
        }

        Ok(())
    }

    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        // Unfreeze the asset
        UpdatePluginV1Cpi::new(
            ctx.accounts.core_program.to_account_info().as_ref(),
            UpdatePluginV1CpiAccounts {
                asset: ctx.accounts.asset.as_ref(),
                collection: Some(&ctx.accounts.collection),
                payer: &ctx.accounts.payer.to_account_info(),
                authority: Some(&ctx.accounts.update_authority.to_account_info()),
                system_program: &ctx.accounts.system_program.to_account_info(),
                log_wrapper: None,
            },
            UpdatePluginV1InstructionArgs {
                plugin: Plugin::FreezeDelegate(FreezeDelegate {frozen: false}),
            }
        ).invoke()?;

        // Remove the FreezeDelegate Plugin
        RemovePluginV1Cpi::new(
            ctx.accounts.core_program.to_account_info().as_ref(),
            RemovePluginV1CpiAccounts {
                asset: ctx.accounts.asset.as_ref(),
                collection: Some(&ctx.accounts.collection),
                payer: &ctx.accounts.payer.to_account_info(),
                authority: Some(&ctx.accounts.signer.to_account_info()),
                system_program: &ctx.accounts.system_program.to_account_info(),
                log_wrapper: None,
            },
            RemovePluginV1InstructionArgs {
                plugin_type: PluginType::FreezeDelegate,
            }
        ).invoke()?;

        // Save data on the attributes of the Core Asset
        let info = ctx.accounts.asset.to_account_info();
        let data = info.try_borrow_mut_data()?;
        let asset = Asset::deserialize(&data)?;
        
        require!(asset.plugin_list.attributes.is_some(), StakingError::AttributesNotInitialized);

        let mut attribute_list: Vec<Attribute> = Vec::new();
        let mut is_initialized: bool = false;
        let mut staked_time: i64 = 0;

        for attribute in asset.plugin_list.attributes.unwrap().attributes.attribute_list.iter() {
            if attribute.key == "staked" {
                require!(attribute.value != "0", StakingError::NotStaked);
                attribute_list.push(Attribute { key: "staked".to_string(), value: Clock::get()?.unix_timestamp.to_string() });
                staked_time = staked_time
                    .checked_add(Clock::get()?.unix_timestamp.checked_sub(attribute.value.parse::<i64>().map_err(|_| StakingError::InvalidTimestamp)?).ok_or(StakingError::Underflow)?)
                    .ok_or(StakingError::Overflow)?;
                is_initialized = true;
            } else if attribute.key == "staked_time" {
                staked_time = staked_time
                    .checked_add(attribute.value.parse::<i64>().map_err(|_| StakingError::InvalidTimestamp)?)
                    .ok_or(StakingError::Overflow)?;
            } else {
                attribute_list.push(attribute.clone());
            } 
        }

        require!(is_initialized, StakingError::StakingNotInitialized);

        attribute_list.push(Attribute { key: "staked_time".to_string(), value: staked_time.to_string() });

        UpdatePluginV1Cpi::new(
            ctx.accounts.core_program.to_account_info().as_ref(),
            UpdatePluginV1CpiAccounts {
                asset: ctx.accounts.asset.as_ref(),
                collection: Some(&ctx.accounts.collection),
                payer: &ctx.accounts.payer.to_account_info(),
                authority: Some(&ctx.accounts.update_authority.to_account_info()),
                system_program: &ctx.accounts.system_program.to_account_info(),
                log_wrapper: None,
            },
            UpdatePluginV1InstructionArgs {
                plugin: Plugin::Attributes(Attributes{ attribute_list }),
            }
        ).invoke()?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Stake<'info> {
    pub signer: Signer<'info>,
    pub update_authority: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    /// CHECK: this will be checked by core
    pub asset: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: this will be checked by core
    pub collection: UncheckedAccount<'info>,
    #[account(address = core_program_id)]
    /// CHECK: this will be checked by core
    pub core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    pub signer: Signer<'info>,
    pub update_authority: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    /// CHECK: this will be checked by core
    pub asset: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: this will be checked by core
    pub collection: UncheckedAccount<'info>,
    #[account(address = core_program_id)]
    /// CHECK: this will be checked by core
    pub core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[error_code]
pub enum StakingError {
    #[msg("Invalid owner")]
    OwnerMismatched,
    #[msg("Invalid timestamp")]
    InvalidTimestamp,
    #[msg("Already staked")]
    AlreadyStaked,
    #[msg("Not staked")]
    NotStaked,
    #[msg("Staking not initialized")]
    StakingNotInitialized,
    #[msg("Attributes not initialized")]
    AttributesNotInitialized,


    #[msg("Underflow")]
    Underflow,
    #[msg("Overflow")]
    Overflow,
}