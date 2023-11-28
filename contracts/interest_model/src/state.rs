use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::Decimal256;
use cosmwasm_std::{CanonicalAddr, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read};

static KEY_CONFIG: &[u8] = b"config";
static KEY_NEWOWNER: &[u8] = b"newowner";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: CanonicalAddr,
    pub base_rate: Decimal256,
    pub interest_multiplier: Decimal256,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct NewOwnerAddr {
    pub new_owner_addr: CanonicalAddr, 
}

pub fn store_new_owner(storage: &mut dyn Storage, data: &NewOwnerAddr) -> StdResult<()> {
    singleton(storage, KEY_NEWOWNER).save(data)
}

pub fn read_new_owner(storage: &dyn Storage) -> StdResult<NewOwnerAddr> {
    singleton_read(storage, KEY_NEWOWNER).load()
}


pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}
