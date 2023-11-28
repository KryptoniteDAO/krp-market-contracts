use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, StdError, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read};

use cw_storage_plus::Map;
use pyth_sdk_cw::PriceIdentifier;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PythFeederConfig {
    pub price_feed_id: PriceIdentifier,
    pub price_feed_symbol: String,
    pub price_feed_decimal: u32,
    pub is_valid: bool,
    pub check_feed_age: bool,
    pub price_feed_age: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: CanonicalAddr,
    pub pyth_contract: CanonicalAddr,
}

pub const PYTH_FEEDER_CONFIG: Map<String, PythFeederConfig> = Map::new("pyth_feeder_config");

static KEY_CONFIG: &[u8] = b"config";

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

pub fn store_pyth_feeder_config(
    storage: &mut dyn Storage,
    asset: String,
    pyth_feeder_config: &PythFeederConfig,
) -> Result<PythFeederConfig, StdError> {
    PYTH_FEEDER_CONFIG.update(storage, asset, |old| match old {
        Some(_) => Ok(pyth_feeder_config.clone()),
        None => Ok(pyth_feeder_config.clone()),
    })
}

pub fn read_pyth_feeder_config(
    storage: &dyn Storage,
    asset: String,
) -> Result<PythFeederConfig, StdError> {
    let pyth_feeder_config = PYTH_FEEDER_CONFIG
        .may_load(storage, asset)?
        .ok_or_else(|| StdError::generic_err("Pyth feeder config not found"));
    Ok(pyth_feeder_config.unwrap())
}
