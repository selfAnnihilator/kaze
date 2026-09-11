# ADR 0012: DownloadProvider Trait, Slskd Soulseek Integration, and Auto-Import

## Context
A key capability of the local-first intelligent music player is enabling users to acquire tracks discovered outside their local collection (via Phase 7 Wishlist) without relying on paid subscription services or closed ecosystems. Soulseek is the most extensive decentralized open protocol for high-quality audio files.

Integrating file acquisition directly into a desktop application presents several architectural challenges:
1. **Coupling**: Directly hardcoding low-level P2P protocol sockets within the application core creates maintenance friction and makes deterministic automated testing impossible.
2. **Legal & Ethical Design**: The application must **never** engage in automated piracy or background downloading without explicit user consent. Every search and download must be an explicit, user-initiated action.
3. **P2P Daemon Management**: Soulseek P2P connections, port mappings, and queue negotiation are best handled by dedicated, proven local daemons (`slskd`) rather than re-implementing the complex protocol from scratch.
4. **Library Continuity**: When files finish downloading, users should not have to manually locate files and run manual imports; the system should seamlessly update the database, verify audio tags, and transition the wishlist status to `DOWNLOADED`.

## Options Considered
1. **Embedded Custom Soulseek Protocol Stack**:
   - Rejected because re-implementing the raw Soulseek socket protocol (obfuscated handshake, distributed peer search, UPnP port forwarding) would balloon the codebase, introduce severe network vulnerabilities, and compromise player stability.
2. **Third-Party Commercial Download Cloud API**:
   - Rejected because it violates the project's zero-cost, subscription-free, local-first philosophy.
3. **Pluggable `DownloadProvider` Trait + Local Slskd Daemon Bridge + Auto-Import Pipeline**:
   - Define a pure async `DownloadProvider` trait (`search`, `start_download`, `get_progress`, `cancel`).
   - Implement `SoulseekProvider` connecting to a locally running Slskd daemon REST API (`http://localhost:5030/api/v0`), gracefully reporting unavailable when the daemon is not running.
   - Implement `MockDownloadProvider` for deterministic headless and CI testing.
   - Implement `DownloadService` to manage task lifecycles in SQLite (`download_tasks` table), broadcast progress events over the `EventBus`, auto-import completed files into the library via `LibraryService`, and transition wishlist records to `DOWNLOADED`.

## Decision
Adopt the **Pluggable `DownloadProvider` Trait with Local Slskd Daemon REST Integration and Automated Library Import Pipeline**.

## Reasoning
* **Separation of Concerns**: Networking and peer negotiations remain isolated in the dedicated Slskd daemon or custom providers.
* **Deterministic Testability**: `MockDownloadProvider` allows 100% test coverage of search, queueing, progress tracking, and error handling without external network dependencies.
* **User Control**: Strictly user-driven. No automated searches or background downloading occur without user initiation.
* **End-to-End Continuity**: Closing the loop between finding music, downloading it, indexing it locally, and updating the wishlist fulfills the application's intelligent local-first purpose.

## Consequences
* Users can search and download high-bitrate music directly through the player if Slskd is installed locally.
* The backend exposes typed commands (`SearchSoulseek`, `StartDownload`, `CancelDownload`, `PollDownloadProgress`) and queries (`GetDownloads`) ready for UI consumption in Phase 9.
