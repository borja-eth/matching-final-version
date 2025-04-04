**Orders**

| **Column** | **Type** | **Nullable** | **Description** |
| --- | --- | --- | --- |
| id | uuid | No |  |
| extId | string | yes | external order id - Client may want to set their own id for order |
| accountId | uuid | No | Account Id |
| type | string | No | limit | market | stop | stoplimit | liquidation | adl |
| instrumentId | uuid | No |  |
| side | string | number | No | buy | sell |
| limitPrice | int64 | No |  |
| triggerPrice | int64 | Yes |  |
| baseAmount | int64 | No | BaseAmount in 10EXP(baseDecimals) |
| remainingQuote | int64 | No |  |
| remainingBase | int64 | No |  |
| filledQuote | int64 | No |  |
| filledBase | int64 | No |  |
| expirationDate | DateTime | No | Order Expiration (2 Years for GTC ) |
| status | string | No | Order Status (see order status recommendations) |
| createdAt | DateTime | No |  |
| updatedAt | DateTime | No |  |
| **triggerBy** | string | Yes | Determines the market price type that will be used to evaluate the trigger price. Options include | LastPrice |  |
| **createdFrom** | string | No | **Api | Front** |

**Order Status**

Order status enum describes the different statuses that might have the order from creation to end due to either cancelling or filling.

| Nombre | Value | Texto | status setter |
| --- | --- | --- | --- |
| PendingNew | PENDING_NEW | The order have been sent but wasn’t been acknowledged yet | Gateway |
| PendingCancel | PENDING_CANCEL | The order is awaiting cancel | Gateway |
| Inactive* | INACTIVE | Order is not working right now ( outside trading hours) | Gateway |
| Rejected | REJECTED | Order rejected by Roxom | Gateway |
| New | NEW | The order is working in the system | Yuta |
| Cancelled | CANCELLED | The order has been cancelled by the system | Moxor |
| PartialFill | PARTIAL_FILL | The order has been filled partially | Moxor |
| Filled | FILLED | Order is completely filled | Moxor |
| WaitingTrigger | WAITING_TRIGGER | Order has been acknowledged by the platform but its not triggerd yet ( i.e order submitted after hours ) | Moxor |
| PartialFillCancelled | PARTIAL_FILL_CANCELLED | Partial Fill Cancelled will be use if there is a maker order partially filled and was cancelled or if there is an IOC order and was partially filled. | Moxor |

**Trade**

| **Column** | **Type** | **Nullable** | **Description** |
| --- | --- | --- | --- |
| id | uuid | No |  |
| instrumentId | uuid | No |  |
| makerOrderId | uuid | No | Maker Order Id |
| takerOrderId | uuid | No | Taker Order Id |
| baseAmount | int64 | No |  |
| quoteAmountInNanoBTC | int64 | No |  |
| createdAt | DateTime | No |  |

**OrderFills**

| Column | Type | Nullable | Description |
| --- | --- | --- | --- |
| Id | uuid | No |  |
| AccountID | uuid | No |  |
| TradeID | uuid | No |  |
| OrderID | uuid | No |  |
| Instrument | uuid | No |  |
| side | string | No |  |
| baseAmount | int64 | No |  |
| quoteAmountInNanoBTC | int64 | No |  |
| feeInNanoBTC | int64 | No |  |
| isTaker | bool  | No |  |
| CreatedAt | timestamp | No |  |