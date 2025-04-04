
+==================================================================================================+
| Category           | Liquibook                       | BorjAI Engine                              |
|-------------------|----------------------------------|---------------------------------------------|
| Order Types       | ✓ Limit Orders                   |  Limit Orders                             |
|                   | ✓ Market Orders                  |  Market Orders                            |
|                   | ✓ Stop Orders                    |  Stop Orders                              |
|                   | ✓ Fill-or-Kill (FOK)            |  FOK Orders                               |
|                   | ✓ Immediate-or-Cancel (IOC)     |  IOC Orders                               |
|-------------------|----------------------------------|---------------------------------------------|
| Order Book        | ✓ Price-Time Priority           | Price-Time Priority                      |
|                   | ✓ Template-based                |  Zero-cost abstractions                   |
|                   | ✓ Header-only                   |  Modular architecture                     |
|                   | ✓ Order Modification            |  Order Modification                       |
|                   | ✓ Order Cancellation            |  Order Cancellation                       |
|-------------------|----------------------------------|---------------------------------------------|
| Depth Management  | ✓ Fixed depth levels            |  Dynamic depth with DashMap               |
|                   | ✓ Level aggregation             |  Real-time aggregation                    |
|                   | ✓ Manual depth tracking         |  Automatic depth updates                  |
|                   | ✓ BBO tracking                  |  BBO tracking with O(1) access            |
|                   | ✓ Iterator support              |  Vector-based depth retrieval             |
|-------------------|----------------------------------|---------------------------------------------|
| Event System      | ✓ Callback-based                |  Lock-free channel based                  |
|                   | ✓ Synchronous                   |  Asynchronous                             |
|                   | ✓ Single listener per type      |  Multiple subscribers                     |
|                   | ✗ No backpressure handling      | Backpressure handling                    |
|                   | ✗ No circuit breaker            |  Circuit breaker pattern                  |
|                   | ✗ No sharding                   |  Sharded event distribution               |
|-------------------|----------------------------------|---------------------------------------------|
| Matching Engine   | ✓ Single-pass matching          |  Single-pass matching                     |
|                   | ✓ Cross prevention              |  Cross prevention                         |
|                   | ✓ Multi-level matching          |  Multi-level matching                     |
|                   | ✓ Stop order triggers           |  Stop order triggers                      |
|-------------------|----------------------------------|---------------------------------------------|
| Performance       | ✓ Template metaprogramming      |  Zero-cost abstractions                   |
|                   | ✓ Callback system               |  Lock-free channels                       |
|                   | ✗ No concurrent depth           |  Concurrent depth with DashMap            |
|                   | ✗ Lock-based synchronization    |  Lock-free where possible                 |
|                   | ✗ No load balancing             |  Event load balancing                     |
|-------------------|----------------------------------|---------------------------------------------|
| Market Data       | ✓ Level 2 data                  |  Level 2 data                            |
|                   | ✓ Trade data                    |  Trade data                              |
|                   | ✓ Order flow                    |  Order flow                              |
|                   | ✓ Market statistics             | Market statistics                       |
|-------------------|----------------------------------|---------------------------------------------|
| API & Integration | ✓ C++ templates                 |  REST API                                |
|                   | ✗ No network interface          |  WebSocket support                       |
|                   | ✗ No real-time streaming        |  Real-time event streaming               |
|                   | ✗ No market maker               |  Market maker simulation                 |
|-------------------|----------------------------------|---------------------------------------------|
| Unique Features   | • Template metaprogramming      | •           |
|                   | • Header-only design            | •        |
|                   | • C++ STL integration           | •             |
|                   | • Compile-time polymorphism     | •              |
|                   |                                 | •                 |
|                   |                                 | •                   |
+==================================================================================================+