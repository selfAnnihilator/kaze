# ADR 0003: Use SQLite with sqlx for Local Persistence

## Context
The music player requires reliable, fast, relational persistence for tens of thousands of tracks, detailed listening history logs, multi-window rankings, taste scores, and full-text search without requiring the user to install, configure, or run a background database server.

## Options Considered
1. **JSON / flat files**: Easy to inspect, but terrible performance on large libraries, lack of ACID guarantees, prone to corruption on sudden system crashes, and no query/indexing capabilities.
2. **Embedded Key-Value stores (RocksDB / sled)**: Fast key-value lookups, but complex querying, no built-in schema migrations, and lacking relational joins needed for playlists, history aggregations, and ranking calculations.
3. **Client-server DBMS (PostgreSQL / MySQL)**: Overkill for a local desktop player; unacceptable requirement for desktop users to maintain external database services.
4. **SQLite via sqlx**: Zero-configuration, battle-tested embedded database, file-based, full ACID support, built-in FTS5 search module, and `sqlx` provides compile-time and runtime migration management with async connection pooling.

## Decision
Use **SQLite** as the primary storage engine, accessed via **`sqlx`**.

## Reasoning
* Zero user setup: single file database located in standard application data paths.
* Outstanding read performance: indices enable sub-millisecond retrieval of tracks, playlists, and history.
* FTS5 provides instant fuzzy/partial text search without introducing Elasticsearch or external indexers.
* WAL mode (`PRAGMA journal_mode = WAL`) guarantees concurrent non-blocking reads while writing.

## Consequences
* Database access is encapsulated strictly within the `database::repositories` layer.
* Schema evolution is handled through versioned SQL migrations embedded into the binary.
