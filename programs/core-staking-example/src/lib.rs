use anchor_lang::prelude::*;
use mpl_core::{
    ID as core_program_id,
    Asset,
    instructions::{ AddPluginV1CpiBuilder, UpdatePluginV1CpiBuilder, RemovePluginV1CpiBuilder },
    types::{FreezeDelegate, Plugin, PluginType, PluginAuthority, Attribute, Attributes}
};

declare_id!("2X7QEU55T9km1ASAixhJpGfFFVFcpfyC9LDwmDgsitaT");

#[program]
pub mod jungle_core_staking {
    use super::*; 

    

    pub fn stake(ctx: Context<Stake>) -> Result<()> {  
        
            
        // Freeze the asset  
        AddPluginV1CpiBuilder::new(&ctx.accounts.core_program.to_account_info())
            .asset(&ctx.accounts.asset.to_account_info())
            .collection(Some(&ctx.accounts.collection.to_account_info()))
            .payer(&ctx.accounts.payer.to_account_info())
            .authority(Some(&ctx.accounts.signer.to_account_info()))
            .system_program(&ctx.accounts.system_program.to_account_info())
            .plugin(Plugin::FreezeDelegate( FreezeDelegate{ frozen: true } ))
            .init_authority(PluginAuthority::UpdateAuthority)
            .invoke()?;
        
        // Save data on the attributes of the Core Asset
        let info = ctx.accounts.asset.to_account_info();
        let data = info.try_borrow_mut_data()?;
        let asset = Asset::deserialize(&data)?;

        drop(data);
        
        if asset.plugin_list.attributes.is_none() {
            AddPluginV1CpiBuilder::new(&ctx.accounts.core_program.to_account_info())
                .asset(&ctx.accounts.asset.to_account_info())
                .collection(Some(&ctx.accounts.collection.to_account_info()))
                .payer(&ctx.accounts.payer.to_account_info())
                .authority(Some(&ctx.accounts.update_authority.to_account_info()))
                .system_program(&ctx.accounts.system_program.to_account_info())
                .plugin(Plugin::Attributes(
                    Attributes{ 
                        attribute_list: vec![
                            Attribute { key: "frozen".to_string(), value: "1".to_string()},
                            Attribute { key: "staked".to_string(), value: Clock::get()?.unix_timestamp.to_string() }, //timestamp now
                            Attribute { key: "staked_time".to_string(), value: 0.to_string() }, //timestamp 0
                        ] 
                    }
                ))
                .init_authority(PluginAuthority::UpdateAuthority)
                .invoke()?;
        } else {
            let mut attribute_list: Vec<Attribute> = Vec::new();
            let mut is_initialized: bool = false;

            for attribute in asset.plugin_list.attributes.unwrap().attributes.attribute_list {
                if attribute.key == "frozen" {
                    require!(attribute.value == "0", StakingError::OwnerMismatched);
                    attribute_list.push(Attribute { key: "frozen".to_string(), value: "1".to_string() });
                } else if attribute.key == "staked" {
                    
                    attribute_list.push(Attribute { key: "staked".to_string(), value: Clock::get()?.unix_timestamp.to_string() });
                    is_initialized = true;
                } else {
                    attribute_list.push(attribute);
                } 
            }

            if is_initialized == false {
                attribute_list.push(Attribute { key: "frozen".to_string(), value: "1".to_string() });
                attribute_list.push(Attribute { key: "staked".to_string(), value: Clock::get()?.unix_timestamp.to_string() });
                attribute_list.push(Attribute { key: "staked_time".to_string(), value: 0.to_string() });
            }

            UpdatePluginV1CpiBuilder::new(&ctx.accounts.core_program.to_account_info())
                .asset(&ctx.accounts.asset.to_account_info())
                .collection(Some(&ctx.accounts.collection.to_account_info()))
                .payer(&ctx.accounts.payer.to_account_info())
                .authority(Some(&ctx.accounts.update_authority.to_account_info()))
                .system_program(&ctx.accounts.system_program.to_account_info())
                .plugin(Plugin::Attributes(Attributes{ attribute_list }))
                .invoke()?;
        }

        //Asset::serialize(&asset, &mut data)?;

        Ok(())
    }

    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        // Unfreeze the asset
        UpdatePluginV1CpiBuilder::new(&ctx.accounts.core_program.to_account_info())
            .asset(&ctx.accounts.asset.to_account_info())
            .collection(Some(&ctx.accounts.collection.to_account_info()))
            .payer(&ctx.accounts.payer.to_account_info())
            .authority(Some(&ctx.accounts.update_authority.to_account_info()))
            .system_program(&ctx.accounts.system_program.to_account_info())
            .plugin(Plugin::FreezeDelegate( FreezeDelegate{ frozen: false } ))
            .invoke()?;

        // Remove the FreezeDelegate Plugin
        RemovePluginV1CpiBuilder::new(&ctx.accounts.core_program)
            .asset(&ctx.accounts.asset)
            .collection(Some(&ctx.accounts.collection))
            .payer(&ctx.accounts.payer)
            .authority(Some(&ctx.accounts.signer))
            .system_program(&ctx.accounts.system_program)
            .plugin_type(PluginType::FreezeDelegate)
            .invoke()?;

        // Save data on the attributes of the Core Asset
        let info = ctx.accounts.asset.to_account_info();
        let data = info.try_borrow_mut_data()?;
        let asset = Asset::deserialize(&data)?;

        drop(data);
        
        require!(asset.plugin_list.attributes.is_some(), StakingError::AttributesNotInitialized);

        let mut attribute_list: Vec<Attribute> = Vec::new();
        let mut is_initialized: bool = false;
        let mut staked_time: i64 = 0;

        for attribute in asset.plugin_list.attributes.unwrap().attributes.attribute_list.iter() {
            if attribute.key == "frozen" {
                require!(attribute.value == "1", StakingError::NotStaked);
                
            } else if attribute.key == "staked" {

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

        attribute_list.push(Attribute { key: "frozen".to_string(), value: "0".to_string() });
        attribute_list.push(Attribute { key: "staked_time".to_string(), value: staked_time.to_string() });

        UpdatePluginV1CpiBuilder::new(&ctx.accounts.core_program.to_account_info())
            .asset(&ctx.accounts.asset.to_account_info())
            .collection(Some(&ctx.accounts.collection.to_account_info()))
            .payer(&ctx.accounts.payer.to_account_info())
            .authority(Some(&ctx.accounts.update_authority.to_account_info()))
            .system_program(&ctx.accounts.system_program.to_account_info())
            .plugin(Plugin::Attributes(Attributes{ attribute_list }))
            .invoke()?;

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