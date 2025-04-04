# Roadmap: High-Performance Rust Matching Engine

This document outlines the phased development plan for building a high-performance, robust matching engine in Rust, adhering to the `@Rust Rules` and best practices.

## Guiding Principles

*   **Performance:** Prioritize low latency and high throughput in the core matching logic. Minimize allocations and copies on the hot path.
*   **Correctness:** Ensure accurate matching based on price-time priority and prevent state corruption. Leverage Rust's type system.
*   **Robustness:** Implement comprehensive error handling, graceful degradation, and thorough testing.
*   **Maintainability:** Write clean, well-documented, modular code following Rust idioms and the `@Rust Rules`.
*   **Simplicity First:** Start with the core logic and incrementally add features and complexity. "You Aren't Going to Need It" (YAGNI).
*   **Adherence to `@Rust Rules`:** Strictly follow the provided rule sets (`always.mdc`, `code_style.mdc`, `golden_rules.mdc`, `matching_engine.mdc`, `matching_engine_perf.mdc`).
*   **Adherence to `@roxom.md`:** Ensure the engine can handle the specified `Order` structure as input and produce `Trade` structures and relevant `Order Status` updates as output.

## Core Technologies & Libraries

*   **Language:** Rust (latest stable)
*   **Async Runtime:** `tokio` (for I/O, task management, channels)
*   **Data Structures:**
    *   `std::collections::BTreeMap`: Initial choice for price level ordering (bids/asks). Offers good balance of performance and simplicity.
    *   `std::collections::VecDeque`: For FIFO order queue within a price level.
    *   `std::collections::HashMap`: For fast `OrderId` lookups.
    *   Potentially explore alternatives (`indexmap`, custom structures) *only* if benchmarking proves `BTreeMap` is a bottleneck.
*   **Numerical Types:** `rust_decimal` (or potentially scaled `u64`/`i64`) for `Price` and `Quantity` to avoid floating-point errors (`@matching_engine_perf.mdc`).
*   **Error Handling:** `thiserror` (for defining specific error types).
*   **Serialization:** `serde` (with `serde_json` for initial API/testing, potentially `bincode` or others for performance-critical inter-process communication later).
*   **Logging/Tracing:** `tracing` framework (with `tracing-subscriber`).
*   **Testing:** Built-in `#[test]`, `criterion` (for benchmarking).
*   **Code Quality:** `clippy`, `rustfmt`.

## Development Phases

**Phase 0: Setup & Core Types (`types.rs`, `errors.rs`)**

*   **Goal:** Establish project structure and define fundamental data types and errors.
*   **Tasks:**
    *   Initialize `cargo` project (likely a library `cargo new --lib ultimate-matching`).
    *   Set up `src/lib.rs` and initial module files (`types.rs`, `errors.rs`, `orderbook.rs`, etc.).
    *   Define core structs (`Order`, `Trade`), enums (`Side`, `OrderStatus` - mapped from `@roxom.md`), and newtypes (`Price`, `Quantity`, `OrderId`, `InstrumentId`, `AccountId`, etc.) in `types.rs`. Ensure compatibility with `@roxom.md` field types (e.g., `uuid`, `i64`). Use `rust_decimal` or scaled integers.
    *   Define basic error enums (`OrderBookError`) in `errors.rs` using `thiserror`.
    *   Add initial dependencies (`uuid`, `rust_decimal`, `thiserror`) to `Cargo.toml`.
    *   Configure `clippy` and `rustfmt`.
    *   Implement basic unit tests for type conversions/validation.
*   **Tech:** `Rust`, `Cargo`, `uuid`, `rust_decimal`, `thiserror`.

**Phase 1: Core Order Book Logic (`orderbook.rs`)**

*   **Goal:** Implement the core, single-instrument order book logic with matching.
*   **Tasks:**
    *   Define `OrderBook` struct in `orderbook.rs`.
    *   Implement internal storage using `BTreeMap<Price, VecDeque<Order>>` for bids/asks and `HashMap<OrderId, Order>` for lookup.
    *   Implement `add_order` method:
        *   Handles `limit` orders initially.
        *   Contains the price-time priority matching algorithm.
        *   Checks for crossing orders (compare incoming buy price vs lowest ask, incoming sell vs highest bid).
        *   If match occurs, generate `Trade` objects.
        *   Update involved orders' `remainingBase`, `filledBase`, `filledQuote`.
        *   Update `OrderStatus` (e.g., `PartialFill`, `Filled`).
        *   Add remaining quantity (if any) to the book.
        *   Return `Result<Vec<Trade>, OrderBookError>`.
    *   Implement `cancel_order` method:
        *   Remove order from `BTreeMap` and `HashMap`.
        *   Update `OrderStatus` to `Cancelled` or `PartialFillCancelled`.
        *   Return `Result<Order, OrderBookError>`.
    *   Write extensive unit tests (`#[cfg(test)]`) covering:
        *   Adding bids/asks.
        *   Simple fills (one level).
        *   Partial fills.
        *   Multi-level fills.
        *   Cancellation (full, partial).
        *   Edge cases (zero quantity, self-match attempts - if applicable).
*   **Tech:** `Rust`, `std::collections`, `uuid`, `rust_decimal`.

**Phase 2: Market Depth (`depth.rs`)**

*   **Goal:** Provide functionality to query the state of the order book.
*   **Tasks:**
    *   Define structs representing market depth levels (e.g., `DepthLevel { price, quantity }`).
    *   Implement functions in `depth.rs` that take an `&OrderBook` and return aggregated depth information (e.g., top N levels, full book snapshot).
    *   Write unit tests for depth generation.
*   **Tech:** `Rust`.

**Phase 3: Basic Matching Engine Orchestration (`matching_engine.rs`, `commands.rs`, `events.rs`)**

*   **Goal:** Create a coordinator that can manage one or more order books and handle basic commands/events (still single-threaded).
*   **Tasks:**
    *   Define `EngineCommand` enum (e.g., `AddOrder(Order)`, `CancelOrder(OrderId, InstrumentId)`) in `commands.rs`.
    *   Define `EngineEvent` enum (e.g., `TradeExecuted(Trade)`, `OrderAccepted(OrderId)`, `OrderCancelled(OrderId)`, `OrderRejected{order_id, reason}`, `OrderStatusUpdate(OrderId, OrderStatus)`) in `events.rs`. Map statuses to `@roxom.md`.
    *   Define `MatchingEngine` struct in `matching_engine.rs`.
    *   Implement internal storage for multiple `OrderBook`s (e.g., `HashMap<InstrumentId, OrderBook>`).
    *   Implement a synchronous `process_command` method on `MatchingEngine` that:
        *   Takes an `EngineCommand`.
        *   Finds/creates the relevant `OrderBook`.
        *   Calls `orderbook.add_order` or `orderbook.cancel_order`.
        *   Translates `OrderBookError` and results into appropriate `EngineEvent`s.
        *   Returns `Vec<EngineEvent>`.
    *   Define `EngineError` in `errors.rs`.
    *   Write integration tests in `tests/` directory simulating command sequences and verifying event output.
*   **Tech:** `Rust`, `thiserror`.

**Phase 4: Async Processing & Communication**

*   **Goal:** Make the engine process commands asynchronously using `tokio`.
*   **Tasks:**
    *   Add `tokio` dependency (with `sync`, `rt-multi-thread`, `macros`).
    *   Refactor `MatchingEngine` to run in its own `tokio` task.
    *   Create `tokio::sync::mpsc` channels: one for sending `EngineCommand`s *to* the engine task, one for broadcasting `EngineEvent`s *from* the engine task.
    *   Modify the engine task to loop, receiving commands from the input channel, processing them via `process_command`, and sending results via the output channel(s).
    *   Create simple "client" tasks that simulate sending commands and receiving events via the channels.
    *   Update integration tests to interact with the engine via channels.
    *   Introduce basic `tracing` for logging engine activity.
*   **Tech:** `tokio`, `tracing`.

**Phase 5: Advanced Order Types & Features**

*   **Goal:** Implement more complex order types and engine capabilities.
*   **Tasks:**
    *   **Market Orders:** Enhance `orderbook.add_order` to handle market orders (match aggressively against available liquidity).
    *   **Stop Orders:** Add logic (likely in `MatchingEngine` or a dedicated module) to monitor market prices (e.g., last trade price) and trigger stop/stop-limit orders when conditions are met, converting them into limit/market orders. Define `triggerBy` logic from `@roxom.md`.
    *   **Time-in-Force (TiF):** Implement basic TiF (GTC handled by book, IOC/FOK within `add_order` logic). Handle `expirationDate`.
    *   **Order Status Refinement:** Ensure all `OrderStatus` transitions from `@roxom.md` are correctly implemented and emitted as events.
    *   **Fee Calculation:** Add basic fee logic during trade generation (placeholder initially).
    *   **Input Validation:** Add robust validation for incoming orders based on `@roxom.md` constraints (e.g., required fields, valid enums).
*   **Tech:** `Rust`, `tokio`.

**Phase 6: Performance Benchmarking & Optimization**

*   **Goal:** Identify and address performance bottlenecks.
*   **Tasks:**
    *   Add `criterion` dependency.
    *   Write benchmarks (`benches/` directory) for critical paths:
        *   High-volume order insertion (adds).
        *   High-volume order cancellation.
        *   Aggressive matching (market orders).
        *   Mixed workload scenarios.
    *   Profile using tools like `perf` or `flamegraph` under benchmark load.
    *   Analyze results, focusing on allocations, hot loops, data structure performance.
    *   **Optimization (if necessary):**
        *   Explore alternative data structures (`HashMap` + heaps, arena allocators).
        *   Reduce cloning/copying on the hot path.
        *   Consider `#[inline]` for small, critical functions (based on profiling).
        *   Evaluate `unsafe` *only* as a last resort for proven, significant bottlenecks, with clear justification and encapsulation (`@matching_engine_perf.mdc`).
*   **Tech:** `criterion`, `perf`, `flamegraph`.

**Phase 7: Persistence & Recovery (Optional)**

*   **Goal:** Allow the engine state to be saved and restored.
*   **Tasks:**
    *   Design a snapshotting mechanism for `OrderBook` state.
    *   Implement serialization (`serde` + `bincode` likely) for the state.
    *   Add commands/logic to trigger snapshots and load from snapshots.
    *   Consider event sourcing as an alternative persistence strategy.
*   **Tech:** `serde`, `bincode` (or other binary format).

**Phase 8: API Layer / Integration**

*   **Goal:** Expose the engine functionality via a network API.
*   **Tasks:**
    *   Choose an API strategy (e.g., gRPC, WebSocket, REST via `axum` or `actix-web`).
    *   Implement API handlers that:
        *   Deserialize incoming requests into `EngineCommand`s.
        *   Send commands to the engine task via its channel.
        *   Subscribe to `EngineEvent`s from the engine's broadcast channel.
        *   Serialize events and send them back to connected clients.
*   **Tech:** `tokio`, `serde`, `axum`/`actix-web`/`tonic`/`tungstenite`.

## Documentation (`code_style.mdc`)

*   Maintain thorough `///` doc comments for all public types, functions, and modules throughout development.
*   Include examples in doc comments where helpful.
*   Keep this `roadmap.md` updated as the project evolves.
*   Implement the ASCII table documentation style at the beginning of files as specified in `code_style.mdc`.

This roadmap provides a clear path from a simple core to a feature-rich, high-performance matching engine, respecting the specified rules and constraints. 