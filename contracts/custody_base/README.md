# Custody Base

The Custody contract is where supplied bAsset collaterals are managed. Users can make collateral
deposits and withdrawals to and from this contract. The Custody contract is also responsible for
claiming bAsset rewards and converting them to Terra stable coins, which are then sent to the [Overseer contract](../overseer) for eventual distribution.
