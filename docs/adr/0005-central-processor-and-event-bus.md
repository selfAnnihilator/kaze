# ADR 0005: Central Processor and Event Bus Dispatching

## Context
A desktop music player involves asynchronous user inputs, continuous hardware audio state transitions, long-running background disk scans, and downstream side-effects (e.g. track finish triggers history logging, which recalculates taste scores, which refreshes smart mixes). Direct circular calling between services leads to lock contention, deadlocks, and brittle coupling.

## Options Considered
1. **Direct Service Invocations**: Services hold references to all other services and invoke callbacks directly. Results in spaghetti references, circular dependencies, and high risk of deadlocks when acquiring locks across service boundaries.
2. **Global Shared State Mutex**: An omniscient state object protected by a single `RwLock`/`Mutex`. Results in severe lock contention, thread starvation, and sluggish UI response during heavy I/O.
3. **Central Processor + Tokio Broadcast Event Bus**:
   - Inbound mutations flow through `CoreProcessor` via typed commands.
   - Outbound notifications are published onto an asynchronous `tokio::sync::broadcast` channel (`EventBus`).
   - Domain services subscribe only to events they care about.
   - Events are mirrored to the UI without blocking backend execution.

## Decision
Implement a **Central Processor** for inbound commands and queries, paired with a **Tokio Broadcast Event Bus** for outbound domain events.

## Reasoning
* Decouples producer from consumer: `PlaybackService` doesn't need to know that `HistoryService`, `StatisticsService`, and `RecommendationEngine` all react to `TrackFinished`.
* Clean UI synchronization: The frontend simply subscribes to the Tauri event stream without polling.
* Prevents deadlocks: Services emit events without holding locks on downstream services.

## Consequences
* Event delivery is broadcast and non-blocking. Senders do not wait for listeners to complete processing.
* Events must be serializable and represent past facts (`TrackFinished`, `ScanCompleted`), not future commands.
