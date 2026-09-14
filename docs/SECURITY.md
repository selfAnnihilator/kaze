# Security Architecture & Credential Management

## 1. Authoritative Cloud Authentication Model

SoundFlow operates a zero-trust, cloud-authoritative authentication system backed by Cloudflare Workers and Cloudflare D1. 

* **No Dual Password Authorities**: Local SQLite never creates, verifies, or persists password hashes. The Cloudflare Worker is the sole authoritative authentication entity.
* **Network Independence**: When the app is offline, previously authenticated sessions remain active via local session metadata until their expiration window concludes. Network errors during login never cause the desktop app to silently fabricate an unverified local account.

---

## 2. Server-Side Password Security & Protection

* **PBKDF2-HMAC-SHA256 with 600,000 Iterations**:
  - Hashing utilizes the native Web Crypto API (`crypto.subtle.deriveBits`) running in C++ within V8 isolates.
  - Exceeds the OWASP recommended 600,000 iterations for PBKDF2-HMAC-SHA256.
  - Each user account has a unique 16-byte cryptographically secure random salt generated via `crypto.getRandomValues`.
* **Timing Attack Defense**:
  - To prevent user enumeration via timing discrepancies, login requests for non-existent usernames execute a full dummy PBKDF2 calculation against a precomputed dummy hash (`DUMMY_HASH`).
  - Response times for valid and invalid usernames are statistically indistinguishable.
* **Rate Limiting**:
  - D1-backed sliding-window rate limiting restricts `/api/auth/login` to 5 requests per minute per IP.
  - Registration is restricted to 3 requests per minute per IP.
  - Exceeded thresholds return HTTP `429 Too Many Requests` with a `Retry-After: 60` header.

---

## 3. Token & Session Hardening

* **Opaque Cryptographic Bearer Tokens**:
  - Tokens are 256-bit cryptographically secure random values (32 bytes formatted as 64-character hexadecimal strings).
  - The raw token is returned to the client exactly once upon successful registration or login.
* **No Raw Token Persistence on Server**:
  - Cloudflare D1 stores only the SHA-256 hash (`token_hash`) of the bearer token in the `sessions` table.
  - Compromise or leak of database backups does not yield usable bearer tokens.
* **Client-Side Credential Storage**:
  - Desktop client stores raw bearer tokens in native operating system credential managers:
    - **Linux**: Secret Service API via DBus (`keyring` crate)
    - **macOS**: Keychain Services
    - **Windows**: Windows Credential Manager
  - **Fallback**: If the OS keyring daemon is locked or unavailable (e.g. headless Linux), the token is written to an access-restricted file (`0600` permissions on Unix) in the application's secure data directory.
  - **SQLite Zero-Leakage**: The local SQLite `cloud_sessions` table stores session metadata only (`user_id`, `username`, `session_id`, `device_id`, `device_name`, `idle_expires_at`, `absolute_expires_at`). Raw tokens are never stored in SQLite.

---

## 4. Hybrid Session Lifetimes & Write Throttling

SoundFlow balances continuous desktop usability with strict bounding and revocation capabilities:

* **Idle Inactivity Timeout**: Sessions expire after 30 days of inactivity (`now >= idle_expires_at`).
* **Absolute Hard Ceiling**: Sessions strictly expire 90 days from creation (`now >= absolute_expires_at`), requiring password re-entry.
* **D1 Write Throttling**:
  - To protect Cloudflare D1 from write amplification and rate-limit exhaustion, `last_used_at` and `idle_expires_at` are updated at most once every 30 minutes on authenticated API calls.
  - The extended idle expiration is clamped: `min(now + 30 days, absolute_expires_at)`.
* **Revocation Integrity**:
  - Every authenticated request checks `revoked_at IS NULL`.
  - Logging out immediately marks the session as revoked (`revoked_at = now`).
  - `POST /api/auth/logout-all` revokes all active sessions across all devices.
  - Remote sessions can be individually inspected and revoked via `GET /api/auth/sessions` and `DELETE /api/auth/sessions/:id`.

---

## 5. Non-Destructive Operations, Access Gating & Sync Security

* **Access Gating vs Data Destruction**:
  - Authentication strictly governs **access permissions**, not data existence.
  - Logging out, session expiration, or remote session revocation purges authentication tokens from secure OS keyrings/fallback files and revokes session records in D1.
  - User-owned SQLite records (custom playlists, playlist tracks, track play stats, likes/dislikes, listening history, downloads) are **strictly preserved** locally.
  - UI queries are gated at the controller level: unauthenticated requests only receive algorithmic Smart Mixes; user-owned data is accessible only when an active authenticated session is verified.
* **Synchronization Security & Tombstones**:
  - Every sync request (`GET /api/sync`, `POST /api/sync`) requires an active, unrevoked bearer token.
  - All cloud operations are strictly isolated by `user_id`; users cannot read or modify another user's playlists, songs, or stats.
  - Sync tombstones ensure intentional deletions are distinguished from local absence, preventing inadvertent remote data wipeouts during multi-device synchronization. Local tombstones are cleared only upon confirmed server synchronization receipt.

---

## 6. Profile Avatar Security & Cloudinary Isolation

* **Strict Session Authority**:
  - Avatar uploads (`POST /api/profile/avatar`) and deletions (`DELETE /api/profile/avatar`) derive identity strictly from the authenticated bearer session token (`auth.user.id`).
  - Client-supplied `user_id` parameters are never accepted as authority for mutations.
* **Payload Sanitization & Size Constraints**:
  - Maximum upload size is strictly capped at 5 MB client-side and server-side.
  - Raw image binaries are parsed and normalized client-side to 256×256 WebP before transmission, eliminating image-based exploits, malicious script embedding, or arbitrary binary storage.
  - Cloudinary public IDs are strictly namespaced server-side: `music-player/avatars/{user_id}` with `overwrite = true` and `invalidate = true`. Path traversal via filenames is impossible.
* **Credentials Security Boundary**:
  - `CLOUDINARY_CLOUD_NAME`, `CLOUDINARY_API_KEY`, and `CLOUDINARY_API_SECRET` reside strictly on the Cloudflare Worker server (in encrypted Worker secrets / `.dev.vars`).
  - Neither the desktop application, the local SQLite database, nor the React frontend ever receive or store the Cloudinary API secret.
  - All upload and destroy operations use SHA-1 signature hashes generated on the Worker.
* **Database Isolation**:
  - D1 stores only profile metadata (`avatar_public_id`, `avatar_url`, `avatar_version`, `avatar_key`, `avatar_updated_at`). Zero image binaries or base64 strings are stored in D1.

