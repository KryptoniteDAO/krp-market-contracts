use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;



#[cw_serde]
pub enum ExecuteMsg {
    Mint { recipient: String, amount: Uint128 },
}