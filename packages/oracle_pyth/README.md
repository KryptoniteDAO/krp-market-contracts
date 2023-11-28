# Pyth oracle

Encapsulate [pyth](https://pyth.network/) for easy use by Kryptonite

## InstantiateMsg {.tabset}

The instantiation message takes in pyth contract address the contract `owner` on pyth oracle.

### Rust

```rust
#[cw_serde]
pub struct InstantiateMsg {
    pub pyth_contract: String,
    pub owner: Addr,
}
```

### JSON

```json
{
  "owner": "sei...address...",
  "pyth_contract": "sei...address..."
}
```

| Key             | Type     | Description                        |
|-----------------|----------|------------------------------------|
| `pyth_contract` | `string` | The pyth contract address.         |
| `owner`         | `string` | The address of the contract owner. |

## ExecuteMsg

### ConfigFeedInfo {.tabset}

Set the relevant information for updating oracle.

#### Rust

```rust
#[cw_serde]
pub enum ExecuteMsg {
    ConfigFeedInfo {
        asset: String,
        price_feed_id: String,
        price_feed_symbol: String,
        price_feed_decimal: u32,
        check_feed_age: bool,
        price_feed_age: u64,
    },
}
```

#### JSON

```json
{
  "config_feed_info": {
    "asset": "factory/sei1h3ukufh4lhacftdf6kyxzum4p86rcnel35v4jk/usdt",
    "price_feed_id": "ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace",
    "price_feed_symbol": "Crypto.ETH/USD",
    "price_feed_decimal": 8,
    "price_feed_age": 360,
    "check_feed_age": true
  }
}
```

| Key                  | Type     | Description                                  |
|----------------------|----------|----------------------------------------------|
| `asset`              | `string` | The asset address.                           |
| `price_feed_id`      | `string` | The pyth price feed id.                      |
| `price_feed_symbol`  | `string` | The pyth price feed symbol.                  |
| `price_feed_decimal` | `u32`    | The pyth price feed decimal.                 |
| `check_feed_age`     | `bool`   | Whether to check the age of the price feed   |
| `price_feed_age`     | `u64`    | The maximum age of the price feed in seconds |

### ChangeOwner {.tabset}

Change the contract `owner`.

#### Rust

```rust
#[cw_serde]
pub enum ExecuteMsg {
    ChangeOwner {
        new_owner: String,
    },
}
```

#### JSON

```json
{
  "change_owner": {
    "new_owner": "sei...addr..."
  }
}
```

| Key         | Type     | Description                            |
|-------------|----------|----------------------------------------|
| `new_owner` | `string` | The address of the new contract owner. |

### SetConfigFeedValid {.tabset}

Set whether the oracle is available, the default is available, the type is bool.

#### Rust

```rust
#[cw_serde]
pub enum ExecuteMsg {
    SetConfigFeedValid {
        asset: String,
        valid: bool,
    },
}
```

#### JSON

```json
{
  "set_config_feed_valid": {
    "asset": "factory/sei1h3ukufh4lhacftdf6kyxzum4p86rcnel35v4jk/usdt",
    "valid": true
  }
}
```

| Key     | Type     | Description                      |
|---------|----------|----------------------------------|
| `asset` | `string` | The asset address.               |
| `valid` | `bool`   | Whether the oracle is available. |

### ChangePythContract {.tabset}

Change the pyth contract address.

#### Rust

```rust
#[cw_serde]
pub enum ExecuteMsg {
    ChangePythContract {
        pyth_contract: String,
    },
}
```

#### JSON

```json
{
  "change_pyth_contract": {
    "pyth_contract": "sei...addr..."
  }
}
```

| Key             | Type     | Description                |
|-----------------|----------|----------------------------|
| `pyth_contract` | `string` | The pyth contract address. |

## QueryMsg

All query messages are described below. A custom struct is defined for each query response.

### QueryPrice {.tabset}

Returns the price of the asset.

#### Rust

```rust
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(PriceResponse)]
    QueryPrice { asset: String },
}
```

#### JSON

```json
{
  "query_price": {
    "asset": "factory/sei1h3ukufh4lhacftdf6kyxzum4p86rcnel35v4jk/usdt"
  }
}
```

| Key     | Type     | Description        |
|---------|----------|--------------------|
| `asset` | `string` | The asset address. |

### PriceResponse {.tabset}

#### Rust

```rust
#[cw_serde]
pub struct PriceResponse {
    pub asset: String,
    pub emv_price: Decimal256,
    pub emv_price_raw: i64,
    pub price: Decimal256,
    pub price_raw: i64,
    pub last_updated_base: u64,
    pub last_updated_quote: u64,
}
```

#### JSON

```json
{
  "price_response": {
    "asset": "factory/sei1h3ukufh4lhacftdf6kyxzum4p86rcnel35v4jk/usdt",
    "emv_price": "1.00000000",
    "emv_price_raw": 100000000,
    "price": "1.00000000",
    "price_raw": 100000000,
    "last_updated_base": 1634160000,
    "last_updated_quote": 1634160000
  }
}
```

| Key                  | Type      | Description                                                                       |
|----------------------|-----------|-----------------------------------------------------------------------------------|
| `asset`              | `string`  | The asset address.                                                                |
| `emv_price`          | `decimal` | The emv price of the asset in the base currency.                                  |
| `emv_price_raw`      | `i64`     | The emv price of the asset in the base currency, multiplied by 10^8.              |
| `price`              | `decimal` | The price of the asset in the quote currency.                                     |
| `price_raw`          | `i64`     | The price of the asset in the quote currency, multiplied by 10^8.                 |
| `last_updated_base`  | `u64`     | The timestamp of the last update of the price of the asset in the base currency.  |
| `last_updated_quote` | `u64`     | The timestamp of the last update of the price of the asset in the quote currency. |

### QueryPrices {.tabset}

Returns the prices of the assets.

#### Rust

```rust
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec < PriceResponse >)]
    QueryPrices { assets: Vec<String> },
}
```

#### JSON

```json
{
  "query_prices": {
    "assets": [
      "factory/sei1h3ukufh4lhacftdf6kyxzum4p86rcnel35v4jk/usdt",
      "factory/sei1h3ukufh4lhacftdf6kyxzum4p86rcnel35v4jk/eth"
    ]
  }
}
```

| Key      | Type       | Description         |
|----------|------------|---------------------|
| `assets` | `[]String` | The asset addresses |

### QueryConfig {.tabset}

Returns information about global config.

#### Rust

```rust
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    QueryConfig {},
}
```

#### JSON

```json
{
  "query_config": {}
}
```

### ConfigResponse {.tabset}

#### Rust

```rust
#[cw_serde]
pub struct PythFeederConfigResponse {
    pub price_feed_id: PriceIdentifier,
    pub price_feed_symbol: String,
    pub price_feed_decimal: u32,
    pub price_feed_age: u64,
    pub check_feed_age: bool,
    pub is_valid: bool,
}
```

#### JSON

```json
{
  "config_response": {
    "price_feed_id": "fff...",
    "price_feed_symbol": "USDT",
    "price_feed_decimal": 6,
    "price_feed_age": 1634160000,
    "check_feed_age": true,
    "is_valid": true
  }
}
```

| Key                  | Type     | Description                          |
|----------------------|----------|--------------------------------------|
| `price_feed_id`      | `string` | The price feed id.                   |
| `price_feed_symbol`  | `string` | The price feed symbol.               |
| `price_feed_decimal` | `u32`    | The price feed decimal.              |
| `price_feed_age`     | `u64`    | The price feed age.                  |
| `check_feed_age`     | `bool`   | Whether to check the price feed age. |
| `is_valid`           | `bool`   | Whether the config is valid.         |

### QueryPythFeederConfig {.tabset}

Returns the pyth feeder config.

#### Rust

```rust
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(PythFeederConfigResponse)]
    QueryPythFeederConfig { asset: String },
}
```

#### JSON

```json
{
  "query_pyth_feeder_config": {
    "asset": "factory/sei1h3ukufh4lhacftdf6kyxzum4p86rcnel35v4jk/usdt"
  }
}
```

| Key     | Type     | Description        |
|---------|----------|--------------------|
| `asset` | `string` | The asset address. |

### PythFeederConfigResponse {.tabset}

#### Rust

```rust
#[cw_serde]
pub struct PythFeederConfigResponse {
    pub price_feed_id: PriceIdentifier,
    pub price_feed_symbol: String,
    pub price_feed_decimal: u32,
    pub price_feed_age: u64,
    pub check_feed_age: bool,
    pub is_valid: bool,
}
```

#### JSON

```json
{
  "pyth_feeder_config_response": {
    "price_feed_id": "fff...",
    "price_feed_symbol": "USDT",
    "price_feed_decimal": 6,
    "price_feed_age": 1634160000,
    "check_feed_age": true,
    "is_valid": true
  }
}
```

| Key                  | Type     | Description                          |
|----------------------|----------|--------------------------------------|
| `price_feed_id`      | `string` | The price feed id.                   |
| `price_feed_symbol`  | `string` | The price feed symbol.               |
| `price_feed_decimal` | `u32`    | The price feed decimal.              |
| `price_feed_age`     | `u64`    | The price feed age.                  |
| `check_feed_age`     | `bool`   | Whether to check the price feed age. |
| `is_valid`           | `bool`   | Whether the config is valid.         |

### QueryExchangeRateByAssetLabel {.tabset}

Returns the exchange rate of the asset label.

#### Rust

```rust
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Decimal256)]
    QueryExchangeRateByAssetLabel { base_label: String, quote_label: String },
}
```

#### JSON

```json
{
  "query_exchange_rate_by_asset_label": {
    "base_label": "USDT",
    "quote_label": "ETH"
  }
}
```

| Key           | Type     | Description      |
|---------------|----------|------------------|
| `base_label`  | `string` | The base label.  |
| `quote_label` | `string` | The quote label. |

