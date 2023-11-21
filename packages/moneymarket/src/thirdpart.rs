use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Uint128, Addr};

#[cw_serde]
pub enum ExecuteMsg {
    VeFundMint { user: Addr, amount: Uint128 },
}
