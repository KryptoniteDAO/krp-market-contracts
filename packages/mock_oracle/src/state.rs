use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::Map;

pub const ORACLE_PRICES: Map<&[u8], i64> = Map::new("mock_oracle_price");

pub fn store_oracle_price(storage: &mut dyn Storage, feed_id: &[u8], price: i64) -> StdResult<()> {
    ORACLE_PRICES.save(storage, feed_id, &price)?;
    Ok(())
}

pub fn read_oracle_price(storage: &dyn Storage, feed_id: &[u8]) -> StdResult<i64> {
    ORACLE_PRICES.load(storage, feed_id)
}
