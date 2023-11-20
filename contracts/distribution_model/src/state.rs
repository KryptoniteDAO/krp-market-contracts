use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::Decimal256;
use cosmwasm_std::{CanonicalAddr, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read};

static KEY_CONFIG: &[u8] = b"config";
const KEY_NEWOWNER: &[u8] = b"newowner";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: CanonicalAddr,
    pub emission_cap: Decimal256,
    pub emission_floor: Decimal256,
    pub increment_multiplier: Decimal256,
    pub decrement_multiplier: Decimal256,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
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