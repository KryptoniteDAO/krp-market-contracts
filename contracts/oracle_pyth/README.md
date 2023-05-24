# Pyth oracle

Encapsulate [pyth](https://pyth.network/) for easy use by Kryptonite

## InstantiateMsg

The instantiation message takes in pyth contract address the contract `owner` on pyth oracle.

```json
{
  "owner": "sei...address...",
  "pyth_contract": "sei...address..."
}
```

## ExecuteMsg

### `config_feed_info`

Set the relevant information for updating oracle.

- `asset` is the asset or denom
- `price_feed_id` is [pyth feed id](https://pyth.network/developers/price-feed-ids)
- `price_feed_symbol` is [pyth feed symbol](https://pyth.network/developers/price-feed-ids)
- `price_feed_decimal`
- `price_feed_age` is the maximum age of the price feed in seconds
- `check_feed_age` is whether to check the age of the price feed

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

### `change_owner`

Change the contract `owner`.

```json
{
  "change_owner": {
    "new_owner": "sei...addr..."
  }
}
```

### `set_config_feed_valid`

Set whether the oracle is available, the default is available, the type is bool.

```json
{
  "set_config_feed_valid": {
    "asset": "factory/sei1h3ukufh4lhacftdf6kyxzum4p86rcnel35v4jk/usdt",
    "valid": true
  }
}
```

## QueryMsg

All query messages are described below. A custom struct is defined for each query response.

### 'query_config'

Returns information about global config.

```json
{
  "query_config": {}
}
```

### 'query_price'

Returns the price of the asset.

```json
{
  "query_price": {
    "asset": "factory/sei1h3ukufh4lhacftdf6kyxzum4p86rcnel35v4jk/usdt"
  }
}
```

### `query_pyth_feeder_config`

Returns the pyth feeder config.

```json
{
  "query_pyth_feeder_config": {
    "asset": "factory/sei1h3ukufh4lhacftdf6kyxzum4p86rcnel35v4jk/usdt"
  }
}
```
