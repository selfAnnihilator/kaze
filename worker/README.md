# SoundFlow Cloudflare Worker & D1 Database

This worker provides cloud-backed user authentication and data synchronization for SoundFlow Music Player.

## Features
- **Server-Side PBKDF2-SHA256 Password Hashing** (100,000 iterations, 16-byte random salt).
- **Secure Token-Based Sessions** (cryptographically random 32-byte tokens).
- **Cloudflare D1 Storage** for:
  - `users`
  - `sessions`
  - `songs`
  - `playlists`
  - `playlist_songs`
  - `song_stats`
  - `user_stats`
  - `user_settings`
- **Zero Client Credential Exposure**: Passwords and hashes never leave the server.

---

## Local Development Setup

1. From the `worker/` directory:
   ```bash
   npm install
   ```

2. Initialize the local D1 SQLite database:
   ```bash
   npm run db:init
   ```

3. Start the local Worker server:
   ```bash
   npm run dev
   ```
   The worker will start on `http://127.0.0.1:8787`.

---

## Deployment to Cloudflare (Free Tier)

1. Log in to your Cloudflare account:
   ```bash
   npx wrangler login
   ```

2. Create the D1 database:
   ```bash
   npx wrangler d1 create soundflow-db
   ```
   Copy the `database_id` output and update `wrangler.jsonc`.

3. Initialize the remote database schema:
   ```bash
   npm run db:init:remote
   ```

4. Deploy the worker:
   ```bash
   npm run deploy
   ```

5. Copy your deployed worker URL (e.g. `https://soundflow-cloud-worker.<subdomain>.workers.dev`) and paste it into the **Settings -> Cloud Sync** field in SoundFlow Desktop!

## Upgrade: exact online recording sync

Existing installations must apply `migrations/0004_song_recording_source.sql` before deploying the updated Worker, then ship the updated desktop client:

```sh
npx wrangler d1 execute soundflow-db --remote --file=./migrations/0004_song_recording_source.sql
npm run deploy
```

Apply this additive migration once to an existing database. Fresh databases initialized from the current `schema.sql` already contain these columns and must not reapply it. Test the migration locally first by replacing `--remote` with `--local` against a disposable existing-schema database.

Songs now carry their original `provider` / `provider_id` separately from an optional `online_source` object (`provider_id`, normalized `metadata`, `linked_at`). D1 stores the latter in `online_source_json`; GET `/api/sync` returns it as an object. Stable Kaze song IDs and many aliases per recording remain intact. Repeated uploads retain the first saved binding; older clients omitting the object cannot clear it.

Backups made before this upgrade cannot reconstruct a converted alias's missing recording on their own. Sync again from a device that still has the saved local binding to fill it in. No title matching or live provider lookup is performed during restore. These commands are rollout instructions; preparing the patch does not deploy it.
