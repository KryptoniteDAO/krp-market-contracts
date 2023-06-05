use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{CanonicalAddr, Deps, Order, StdError, StdResult, Storage};
use cosmwasm_storage::{Bucket, ReadonlyBucket, ReadonlySingleton, Singleton};

use moneymarket::overseer::{CollateralsResponse, WhitelistResponseElem, MarketlistResponseElem,};
use moneymarket::tokens::Tokens;

pub type Market = (CanonicalAddr, String);
pub type Markets = Vec<Market>;

const KEY_CONFIG: &[u8] = b"config";
const PREFIX_DYNRATE_CONFIG: &[u8] = b"dynrate_config";
const PREFIX_EPOCH_STATE: &[u8] = b"epoch_state";
const PREFIX_DYNRATE_STATE: &[u8] = b"dynrate_state";
const PREFIX_MARKET_CONFIG: &[u8] = b"market_config";

const PREFIX_WHITELIST: &[u8] = b"whitelist";
const PREFIX_COLLATERALS: &[u8] = b"collateral";

static PREFIX_MARKETLIST: &[u8] = b"marketlist";



#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner_addr: CanonicalAddr,
    pub oracle_contract: CanonicalAddr,
    pub liquidation_contract: CanonicalAddr,
    pub collector_contract: CanonicalAddr,
    pub epoch_period: u64,
    pub krp_purchase_factor: Decimal256,
    pub price_timeframe: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MarketlistElem {
    pub market_contract:CanonicalAddr, 
    pub stable_denom: String,
    pub stable_name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MarketConfig {
    pub threshold_deposit_rate: Decimal256,
    pub target_deposit_rate: Decimal256,
    pub buffer_distribution_factor: Decimal256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DynrateConfig {
    pub dyn_rate_epoch: u64,
    pub dyn_rate_maxchange: Decimal256,
    pub dyn_rate_yr_increase_expectation: Decimal256,
    // clamps the deposit rate (in blocks)
    pub dyn_rate_min: Decimal256,
    pub dyn_rate_max: Decimal256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EpochState {
    pub deposit_rate: Decimal256,
    pub prev_atoken_supply: Uint256,
    pub prev_exchange_rate: Decimal256,
    pub prev_interest_buffer: Uint256,
    pub last_executed_height: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DynrateState {
    pub last_executed_height: u64,
    pub prev_yield_reserve: Decimal256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WhitelistElem {
    pub name: String,
    pub symbol: String,
    pub max_ltv: Decimal256,
    pub custody_contract: CanonicalAddr,
}

pub fn store_config(storage: &mut dyn Storage, data: &Config) -> StdResult<()> {
    Singleton::new(storage, KEY_CONFIG).save(data)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    ReadonlySingleton::new(storage, KEY_CONFIG).load()
}

pub fn store_market_config(
    storage: &mut dyn Storage,
    market_contract: &CanonicalAddr, 
    market_config: &MarketConfig,
) -> StdResult<()> {
    let mut market_config_bucket: Bucket<MarketConfig> = Bucket::new(storage, PREFIX_MARKET_CONFIG);
    market_config_bucket.save(&market_contract.as_slice(), market_config)?;

    Ok(())
}

pub fn read_market_config(
    storage: &dyn Storage,
    market_contract: &CanonicalAddr,
)-> StdResult<MarketConfig> {
    let market_config_bucket: ReadonlyBucket<MarketConfig> = ReadonlyBucket::new(storage, PREFIX_MARKET_CONFIG);
    match market_config_bucket.load(&market_contract.as_slice()) {
        Ok(v) => Ok(v),
        _ => Err(StdError::generic_err("Market is not configured")),
    }
}

pub fn store_market_dynrate_config(
    storage: &mut dyn Storage,
    market_contract: &CanonicalAddr, 
    dynrate_config: &DynrateConfig,
) -> StdResult<()> {
    let mut dynrate_config_bucket: Bucket<DynrateConfig> = Bucket::new(storage, PREFIX_DYNRATE_CONFIG);
    dynrate_config_bucket.save(&market_contract.as_slice(), dynrate_config)?;

    Ok(())
}

pub fn read_market_dynrate_config(
    storage: &dyn Storage,
    market_contract: &CanonicalAddr,
) -> StdResult<DynrateConfig> {
    let dynrate_config_bucket: ReadonlyBucket<DynrateConfig> =  ReadonlyBucket::new(storage, PREFIX_DYNRATE_CONFIG);
    match dynrate_config_bucket.load(&market_contract.as_slice()) {
        Ok(v) => Ok(v),
        _ => Err(StdError::generic_err("Market is not configured dynrate parameters"
        )),
    }       
}

pub fn store_market_epoch_state(
    storage: &mut dyn Storage, 
    market_contract: &CanonicalAddr,
    epoch_state: &EpochState,
) -> StdResult<()> {
    let mut epoch_state_bucket: Bucket<EpochState> = Bucket::new(storage, PREFIX_EPOCH_STATE);
    epoch_state_bucket.save(&market_contract.as_slice(), epoch_state)?;
    Ok(())
}

pub fn read_market_epoch_state(
    storage: &dyn Storage, 
    market_contract: &CanonicalAddr,
)->StdResult<EpochState> {
    let epoch_state_bucket: ReadonlyBucket<EpochState> = ReadonlyBucket::new(storage, PREFIX_EPOCH_STATE);
    match epoch_state_bucket.load(&market_contract.as_slice()) {
        Ok(v)=> Ok(v),
        _ => Err(StdError::generic_err("Read market epoch state failure")),
    }
}

pub fn store_market_dynrate_state(
    storage: &mut dyn Storage, 
    market_contract: &CanonicalAddr, 
    dynrate_state: &DynrateState,
) -> StdResult<()> {
    let mut dynrate_state_bucket: Bucket<DynrateState> = Bucket::new(storage, PREFIX_DYNRATE_STATE);
    dynrate_state_bucket.save(&market_contract.as_slice(), dynrate_state)?;

    Ok(())
}

pub fn read_market_dynrate_state(
    storage: &dyn Storage,
    market_contract: &CanonicalAddr,
) -> StdResult<DynrateState> {
    let dynrate_state_bucket: ReadonlyBucket<DynrateState> = ReadonlyBucket::new(storage, PREFIX_DYNRATE_STATE);
    match dynrate_state_bucket.load(&market_contract.as_slice()) {
        Ok(v) => Ok(v),
        _ => Err(StdError::generic_err("Read market dynrate state failure")),
    }
}

pub fn store_whitelist_elem(
    storage: &mut dyn Storage,
    collateral_token: &CanonicalAddr,
    whitelist_elem: &WhitelistElem,
) -> StdResult<()> {
    let mut whitelist_bucket: Bucket<WhitelistElem> = Bucket::new(storage, PREFIX_WHITELIST);
    whitelist_bucket.save(collateral_token.as_slice(), whitelist_elem)?;

    Ok(())
}

pub fn read_whitelist_elem(
    storage: &dyn Storage,
    collateral_token: &CanonicalAddr,
) -> StdResult<WhitelistElem> {
    let whitelist_bucket: ReadonlyBucket<WhitelistElem> =
        ReadonlyBucket::new(storage, PREFIX_WHITELIST);
    match whitelist_bucket.load(collateral_token.as_slice()) {
        Ok(v) => Ok(v),
        _ => Err(StdError::generic_err(
            "Token is not registered as collateral",
        )),
    }
}

pub fn read_whitelist(
    deps: Deps,
    start_after: Option<CanonicalAddr>,
    limit: Option<u32>,
) -> StdResult<Vec<WhitelistResponseElem>> {
    let whitelist_bucket: ReadonlyBucket<WhitelistElem> =
        ReadonlyBucket::new(deps.storage, PREFIX_WHITELIST);

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after);

    whitelist_bucket
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|elem| {
            let (k, v) = elem?;
            let collateral_token = deps.api.addr_humanize(&CanonicalAddr::from(k))?.to_string();
            let custody_contract = deps.api.addr_humanize(&v.custody_contract)?.to_string();
            Ok(WhitelistResponseElem {
                name: v.name,
                symbol: v.symbol,
                collateral_token,
                custody_contract,
                max_ltv: v.max_ltv,
            })
        })
        .collect()
}

pub fn store_marketlist_elem(
    storage: &mut dyn Storage,
    market_contract: &CanonicalAddr,
    marketlist_elem: &MarketlistElem,
) -> StdResult<()> {
    let mut marketlist_bucket: Bucket<MarketlistElem> = Bucket::new(storage, PREFIX_MARKETLIST);
    marketlist_bucket.save(&market_contract.as_slice(), marketlist_elem)?;

    Ok(())
}

pub fn read_marketlist_elem(
    storage: &dyn Storage,
    market_contract: &CanonicalAddr,
) ->StdResult<MarketlistElem> {
    let marketlist_bucket: ReadonlyBucket<MarketlistElem> = 
        ReadonlyBucket::new(storage, PREFIX_MARKETLIST);
        match marketlist_bucket.load(&market_contract.as_slice()) {
            Ok(v) => Ok(v),
            _=> Err(StdError::generic_err("Market contract is not added to list",
            )),    
        }
}


pub fn read_all_marketlist(
   deps: Deps, 
   start_after: Option<CanonicalAddr>,
   limit: Option<u32>,
) -> StdResult<Vec<MarketlistResponseElem>> {
    let marketlist_bucket: ReadonlyBucket<MarketlistElem> = 
        ReadonlyBucket::new(deps.storage, PREFIX_MARKETLIST);

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after);

    marketlist_bucket
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|elem| {
            let (k, v) = elem?;
            let market_contract = deps.api.addr_humanize(&CanonicalAddr::from(k))?.to_string();
            Ok(MarketlistResponseElem {
                market_contract,
                stable_denom : v.stable_denom,
                stable_name: v.stable_name,
            })
        })
        .collect()
}

#[allow(clippy::ptr_arg)]
pub fn store_collaterals(
    storage: &mut dyn Storage,
    borrower: &CanonicalAddr,
    collaterals: &Tokens,
) -> StdResult<()> {
    let mut collaterals_bucket: Bucket<Tokens> = Bucket::new(storage, PREFIX_COLLATERALS);
    if collaterals.is_empty() {
        collaterals_bucket.remove(borrower.as_slice());
    } else {
        collaterals_bucket.save(borrower.as_slice(), collaterals)?;
    }

    Ok(())
}

pub fn read_collaterals(storage: &dyn Storage, borrower: &CanonicalAddr) -> Tokens {
    let collaterals_bucket: ReadonlyBucket<Tokens> =
        ReadonlyBucket::new(storage, PREFIX_COLLATERALS);
    match collaterals_bucket.load(borrower.as_slice()) {
        Ok(v) => v,
        _ => vec![],
    }
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn read_all_collaterals(
    deps: Deps,
    start_after: Option<CanonicalAddr>,
    limit: Option<u32>,
) -> StdResult<Vec<CollateralsResponse>> {
    let whitelist_bucket: ReadonlyBucket<Tokens> =
        ReadonlyBucket::new(deps.storage, PREFIX_COLLATERALS);

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after);

    whitelist_bucket
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|elem| {
            let (k, v) = elem?;
            let borrower = deps.api.addr_humanize(&CanonicalAddr::from(k))?.to_string();
            let collaterals: Vec<(String, Uint256)> = v
                .iter()
                .map(|c| Ok((deps.api.addr_humanize(&c.0)?.to_string(), c.1)))
                .collect::<StdResult<Vec<(String, Uint256)>>>()?;

            Ok(CollateralsResponse {
                borrower,
                collaterals,
            })
        })
        .collect()
}

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_start(start_after: Option<CanonicalAddr>) -> Option<Vec<u8>> {
    start_after.map(|addr| {
        let mut v = addr.as_slice().to_vec();
        v.push(1);
        v
    })
}
