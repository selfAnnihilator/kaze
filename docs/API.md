# API Specification & IPC Interface

## 1. Cloudflare Worker HTTP REST Endpoints

All authenticated endpoints require the header `Authorization: Bearer <raw_token>`.

### 1.1 Authentication & Session Management

| Method | Path | Auth Required | Description |
| :--- | :--- | :---: | :--- |
| `POST` | `/api/auth/register` | No | Creates a user account and initiates a session. Returns raw token and session metadata. |
| `POST` | `/api/auth/login` | No | Authenticates username & password. Returns raw token and session metadata. |
| `POST` | `/api/auth/logout` | Yes | Revokes the current session (`revoked_at = now, revoked_reason = 'user_logout'`). |
| `POST` | `/api/auth/logout-all` | Yes | Revokes all active sessions for the authenticated user. |
| `GET` | `/api/auth/sessions` | Yes | Returns all active sessions for the user with `is_current: true` for calling token. |
| `DELETE` | `/api/auth/sessions/:id` | Yes | Revokes a specific session owned by the authenticated user. |
| `GET` | `/api/auth/me` | Yes | Validates session token and returns the current user profile. |

#### Session Object Schema
```json
{
  "id": "sess_uuid_12345",
  "device_id": "dev_uuid_67890",
  "device_name": "Linux Desktop",
  "client_version": "0.1.0",
  "created_at": 1789380000,
  "last_used_at": 1789381800,
  "idle_expires_at": 1791972000,
  "absolute_expires_at": 1797156000,
  "is_current": true
}
```

### 1.2 Data Synchronization

| Method | Path | Auth Required | Description |
| :--- | :--- | :---: | :--- |
| `GET` | `/api/sync/pull` | Yes | Pulls user data (`songs`, `playlists`, `playlist_songs`, `song_stats`, `user_stats`, `user_settings`) from D1. |
| `POST` | `/api/sync/push` | Yes | Pushes local data changes into D1. |

---

## 2. Desktop IPC Commands (`execute_command`)

Dispatched from frontend via `invoke("execute_command", { command: { command: string, payload?: object } })`:

| Command | Payload | Description |
| :--- | :--- | :--- |
| `SignUp` | `{ username, password }` | Registers user against authoritative cloud worker. |
| `Login` | `{ username, password }` | Authenticates user and caches session metadata locally. |
| `Logout` | None | Revokes current session on cloud worker and purges local credentials. |
| `LogoutAll` | None | Revokes all active user sessions across all devices and logs out locally. |
| `RevokeSession` | `{ session_id }` | Revokes a specific session remotely. |
| `SyncCloudData` | None | Pulls remote cloud updates and pushes local modifications. |
| `SetCloudServerUrl` | `{ url }` | Updates configured Cloudflare Worker endpoint URL. |
| `PlayTrack` | `{ track_id }` | Starts playback of a track by ID. |
| `PauseTrack` | None | Pauses active playback. |
| `ResumeTrack` | None | Resumes paused playback. |
| `StopPlayback` | None | Stops playback. |
| `SeekPlayback` | `{ position_secs }` | Seeks playback to given timestamp. |
| `SetVolume` | `{ volume }` | Adjusts playback volume (0.0 to 1.0). |
| `CreatePlaylist` | `{ name, description? }` | Creates a new user playlist. |
| `DeletePlaylist` | `{ playlist_id }` | Deletes a user playlist. |
| `AddTrackToPlaylist` | `{ playlist_id, track_id, ... }` | Adds a track to a playlist. |
| `RemoveTrackFromPlaylist` | `{ playlist_id, track_id }` | Removes a track from a playlist. |
| `LikeTrack` | `{ track_id }` | Marks a track as liked. |
| `DislikeTrack` | `{ track_id }` | Marks a track as disliked. |

---

## 3. Desktop IPC Queries (`execute_query`)

Dispatched from frontend via `invoke("execute_query", { query: { query: string, payload?: object } })`:

| Query | Payload | Returns | Description |
| :--- | :--- | :--- | :--- |
| `GetCurrentUser` | None | `UserProfile \| null` | Returns active user profile if session is valid. |
| `GetCloudSyncStatus` | None | `CloudSyncStatus` | Returns connection status, worker URL, device name, and session expiry timestamps. |
| `GetSessionState` | None | `AuthSessionState` | Returns strongly typed session state (`OnlineAuthenticated`, `OfflineAuthenticated`, etc.). |
| `ListSessions` | None | `SessionInfo[]` | Queries Cloudflare Worker for active user sessions. |
| `GetPlaybackState` | None | `PlaybackState` | Current audio playback status and track details. |
| `GetTracks` | `{ offset, limit, sort_by?, ascending }` | `Track[]` | Paginated library tracks. |
| `GetPlaylists` | None | `Playlist[]` | All local user and smart playlists. |
| `GetStatsOverview` | `{ year?, month? }` | `StatsOverview` | Granular listening statistics and top items. |
