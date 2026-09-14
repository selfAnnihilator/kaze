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
