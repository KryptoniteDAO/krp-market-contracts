use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};
use oracle_pyth::msg::{
    ChangeOwnerMsg, ConfigResponse, ExecuteMsg, InstantiateMsg, PriceResponse,
    PythFeederConfigResponse, QueryMsg, SetConfigFeedValidMsg,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(SetConfigFeedValidMsg), &out_dir);
    export_schema(&schema_for!(SetConfigFeedValidMsg), &out_dir);
    export_schema(&schema_for!(ChangeOwnerMsg), &out_dir);
    export_schema(&schema_for!(PriceResponse), &out_dir);
    export_schema(&schema_for!(PythFeederConfigResponse), &out_dir);
    export_schema(&schema_for!(ConfigResponse), &out_dir);
}
