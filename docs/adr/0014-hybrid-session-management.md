# ADR 0014: Hybrid Session Management with Opaque Hashed Tokens, Idle Expiry, and D1 Write Throttling

## Context
SoundFlow uses a serverless cloud backend (Cloudflare Worker + D1 SQL database) for user profile synchronization, playlist backup, and listening history archives. As a desktop application, SoundFlow may run continuously for weeks or remain dormant between listening sessions. The application requires an authentication and session model that is:
1. **Desktop-Friendly**: Does not force repeated password logins during normal active use.
2. **Strictly Bounded**: Enforces an absolute lifetime limit regardless of continuous activity.
3. **Revocable & Multi-Device Aware**: Supports granular session listing, per-session revocation, and global logout-all across all client devices.
4. **D1 Write-Efficient**: Avoids write amplification and rate-limit exhaustion on Cloudflare D1 by avoiding updates on every song play or sync request.
5. **Secure & Zero-Leakage**: The server must never store raw bearer tokens or plaintext passwords, and the local SQLite database must store only non-secret session metadata.
6. **Non-Destructive**: Logging out or session expiration must never delete local playlists, stats, or music files.

## Options Considered

1. **Stateless JSON Web Tokens (JWTs)**:
   - *Pros*: Server doesn't need to look up session state on every read.
   - *Cons*: Difficult to revoke immediately when a user loses a device or clicks "Log Out All Devices" without complex blacklist mechanisms in D1 or KV. Expiration is fixed unless refresh token rotation is introduced, adding complexity.

2. **Simple Infinite or Fixed-Duration Server Sessions**:
   - *Pros*: Minimal implementation complexity.
   - *Cons*: Either leaves sessions open indefinitely if a device is decommissioned, or prematurely interrupts user playback every few days with disruptive login modals.

3. **Hybrid Session Model (30-Day Idle Timeout + 90-Day Absolute Ceiling + D1 Throttled Sliding Window)**:
   - *Pros*:
     - Inactivity timeout (30 days) automatically invalidates abandoned or lost devices.
     - Hard ceiling (90 days) ensures credentials must eventually be renewed with password re-authentication.
     - Throttled D1 updates (at most once every 30 minutes) cap write operations on D1 while extending the idle timeout during active playback and syncing.
     - Opaque 256-bit random tokens with SHA-256 hashing in D1 prevent token leakage from server-side database snapshots.
     - Storage of raw bearer tokens in native OS keyrings (with secure local file fallback) protects credentials at rest on client operating systems.
   - *Cons*: Requires coordinating session timestamps across the Worker and client SQLite cache.

## Decision
We adopt the **Hybrid Session Model with Opaque Hashed Tokens, Idle Expiry, and D1 Write Throttling**:

1. **Token Architecture**:
   - Authentication produces a 256-bit cryptographically secure random hexadecimal token (32 random bytes generated via `crypto.getRandomValues`).
   - The raw bearer token is returned to the client exactly once upon successful registration or login.
   - D1 stores only the cryptographic SHA-256 hash (`token_hash`) in the `sessions` table.
   - Local SQLite stores only non-secret metadata (`session_id`, `device_id`, `device_name`, `idle_expires_at`, `absolute_expires_at`, `last_cloud_validation_at`, `worker_url`).
   - The raw token is stored in native OS credential storage (Secret Service on Linux, Credential Manager on Windows, Keychain on macOS) with restricted `0600` file fallback.

2. **Hybrid Expiry Logic**:
   - **Idle Timeout**: 30 days (`IDLE_TIMEOUT_SECONDS = 30 * 86400`).
   - **Absolute Hard Ceiling**: 90 days from session creation (`ABSOLUTE_TIMEOUT_SECONDS = 90 * 86400`).
   - **Validation Predicate**: A session is valid if and only if:
     `now < idle_expires_at AND now < absolute_expires_at AND revoked_at IS NULL`.
   - **Write Throttling**: On authenticated requests, `last_used_at` and `idle_expires_at` are refreshed in D1 only if `now - last_used_at >= 1800` (30 minutes). The refreshed idle expiration is clamped to never exceed `absolute_expires_at`: `min(now + 30 days, absolute_expires_at)`.

3. **Device Identity**:
   - Each desktop client instance generates a stable random UUID v4 on first launch, persisted in SQLite `application_settings` under key `"device_id"`.
   - Friendly non-invasive device names are inferred from the operating system (e.g., "Linux Desktop", "macOS Laptop", "Windows PC") without fingerprinting hardware.

4. **Multi-Device Session Management**:
   - `POST /api/auth/logout`: Revokes the current session (`revoked_at = now, revoked_reason = 'user_logout'`).
   - `POST /api/auth/logout-all`: Revokes all active sessions for the authenticated user.
   - `GET /api/auth/sessions`: Lists active sessions for the authenticated user, indicating `is_current: true` for the calling token.
   - `DELETE /api/auth/sessions/:id`: Revokes a specific session owned by the authenticated user.
   - Periodic scheduled cleanup removes sessions revoked or expired for more than 30 days.

5. **Client Lifecycle States & Offline Continuation**:
   - Session states are strongly typed: `SignedOut`, `Authenticating`, `OnlineAuthenticated`, `OfflineAuthenticated`, `SessionExpired { reason }`, `CloudUnavailable`, `SyncPaused`.
   - On startup or network outage, if a device has previously authenticated (`authenticated_before == 1`) and the current time is within both local idle and absolute expiry limits, the client enters `OfflineAuthenticated`.
   - Offline usage does not extend session validity; only successful cloud validation advances the idle window.

6. **Data Preservation**:
   - Logout, session expiration, and remote revocation clear user credentials and session rows, but **strictly preserve** local playlists, audio files, play statistics, and download tasks.

## Consequences
- Session state is predictable, secure, and respectful of desktop music listening habits.
- D1 writes are strictly throttled, avoiding database quota issues.
- Credentials cannot be compromised through database dumps or local SQLite inspection.
- Users have full visibility into and control over devices connected to their account.
