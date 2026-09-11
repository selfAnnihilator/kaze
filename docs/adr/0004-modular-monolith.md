# ADR 0004: Modular Monolith Architecture

## Context
Desktop applications frequently suffer from two anti-patterns: either an unmaintainable "spaghetti" codebase where UI code talks directly to databases and audio drivers, or premature overengineering into multiple microservices, background daemons, or separate HTTP/gRPC servers.

## Options Considered
1. **Unstructured Monolith**: Rapid to hack together, but tight coupling between components makes testing, refactoring, and adding providers extremely difficult.
2. **Microservices / Local Micro-daemons**: Running separate processes for audio playback, scanning, and recommendations connected via gRPC/IPC. Introduces immense process lifecycle management, IPC latency, network port collision headaches, and debugging complexity.
3. **Modular Monolith**: Single compiled binary and runtime process, but with strict internal module boundaries, explicit public interfaces, trait-based abstractions, and clean dependency hierarchies.

## Decision
Adopt a **Modular Monolith** architecture in Rust.

## Reasoning
* All domains (Playback, Library, History, Statistics, Recommendations, Metadata) reside in distinct Rust modules with explicit visibility (`pub(crate)` / `pub`).
* Eliminates inter-process synchronization bugs and network overhead.
* Simplifies packaging, deployment, and cross-platform installation into a single executable.
* Unit and integration testing are fast and can test components in isolation using mock traits.

## Consequences
* Boundaries must be rigorously maintained through code reviews and compiler visibility checks.
* Modules must not cross-import concrete internals of other modules; interactions must pass through service interfaces or the event bus.
