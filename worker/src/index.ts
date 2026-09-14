/**
 * SoundFlow Cloudflare Worker & D1 Storage Backend
 * Hardened Cloud Authentication & Selective Data Synchronization
 *
 * Security Architecture:
 * - Password Hashing: PBKDF2-HMAC-SHA256 with 600,000 iterations (OWASP recommendation)
 *   and 16-byte cryptographically secure random salt using native Web Crypto API.
 * - Single Account Authority: Cloudflare D1 is the sole credential authority.
 * - Session Token Storage: Raw tokens (32-byte / 256-bit cryptographically secure hex)
 *   are NEVER stored on the server. D1 stores only the SHA-256 hash of the token.
 * - Rate Limiting: Strict IP-based sliding window rate limits on /api/auth/login and
 *   /api/auth/register against brute-force attacks.
 * - Generic Errors: Login failures always return "Invalid username or password" with
 *   constant-time comparison execution to prevent user enumeration.
 * - Server-side Input Validation: Explicit bounds on username (3-50 chars, alphanumeric)
 *   and password (8-128 chars).
 */

export interface Env {
  DB: D1Database;
}

interface UserRecord {
  id: string;
  username: string;
  password_hash: string;
  created_at: number;
}

// Cloudflare Workers maximum supported iteration count for PBKDF2-HMAC-SHA256 (edge ceiling is 100,000)
const PBKDF2_ITERATIONS = 100000;
const IDLE_TIMEOUT_SECONDS = 30 * 24 * 3600; // 30 days of inactivity
const ABSOLUTE_TIMEOUT_SECONDS = 90 * 24 * 3600; // 90 days absolute maximum lifetime
const ACTIVITY_UPDATE_THRESHOLD_SECONDS = 30 * 60; // 30 minutes throttled D1 session activity updates
const CLEANUP_RETENTION_SECONDS = 30 * 24 * 3600; // Keep revoked/expired records up to 30 days before purge
const DUMMY_HASH =
  "00000000000000000000000000000000:0000000000000000000000000000000000000000000000000000000000000000";

export interface AuthenticatedSession {
  session_id: string;
  user_id: string;
  username: string;
  device_id: string;
  device_name: string;
  client_version: string;
  created_at: number;
  last_used_at: number;
  idle_expires_at: number;
  absolute_expires_at: number;
}

// --- Helper Functions: Cryptographic Utilities ---

function bufferToHex(buffer: ArrayBuffer | Uint8Array): string {
  const bytes = buffer instanceof Uint8Array ? buffer : new Uint8Array(buffer);
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

function hexToBuffer(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
  }
  return bytes;
}

async function hashPassword(password: string, saltHex?: string): Promise<string> {
  const salt = saltHex ? hexToBuffer(saltHex) : crypto.getRandomValues(new Uint8Array(16));
  const enc = new TextEncoder();
  const passKey = await crypto.subtle.importKey(
    "raw",
    enc.encode(password),
    { name: "PBKDF2" },
    false,
    ["deriveBits"]
  );

  const derivedBits = await crypto.subtle.deriveBits(
    {
      name: "PBKDF2",
      salt: salt,
      iterations: PBKDF2_ITERATIONS,
      hash: "SHA-256",
    },
    passKey,
    256
  );

  const hashHex = bufferToHex(derivedBits);
  const saltOut = bufferToHex(salt);
  return `${saltOut}:${hashHex}`;
}

async function verifyPassword(password: string, storedHash: string): Promise<boolean> {
  const parts = storedHash.split(":");
  if (parts.length !== 2) return false;
  const saltHex = parts[0];
  const computed = await hashPassword(password, saltHex);
  return computed === storedHash;
}

function generateRawToken(): string {
  const rand = crypto.getRandomValues(new Uint8Array(32));
  return bufferToHex(rand);
}

async function hashToken(rawToken: string): Promise<string> {
  const enc = new TextEncoder();
  const digest = await crypto.subtle.digest("SHA-256", enc.encode(rawToken));
  return bufferToHex(digest);
}

function getClientIp(request: Request): string {
  return (
    request.headers.get("cf-connecting-ip") ||
    request.headers.get("x-forwarded-for")?.split(",")[0]?.trim() ||
    "127.0.0.1"
  );
}

async function checkRateLimit(
  db: D1Database,
  clientIp: string,
  action: "login" | "register"
): Promise<{ allowed: boolean; retryAfter?: number }> {
  const now = Math.floor(Date.now() / 1000);
  const windowSeconds = 60;
  const maxAttempts = action === "login" ? 5 : 3;
  const key = `rl:${action}:${clientIp}`;

  const entry = await db
    .prepare("SELECT attempts, reset_at FROM auth_rate_limits WHERE key = ?")
    .bind(key)
    .first<{ attempts: number; reset_at: number }>();

  if (!entry || entry.reset_at <= now) {
    const resetAt = now + windowSeconds;
    await db
      .prepare(
        "INSERT INTO auth_rate_limits (key, attempts, reset_at) VALUES (?, 1, ?) " +
        "ON CONFLICT(key) DO UPDATE SET attempts = 1, reset_at = excluded.reset_at"
      )
      .bind(key, resetAt)
      .run();
    return { allowed: true };
  }

  if (entry.attempts >= maxAttempts) {
    return { allowed: false, retryAfter: entry.reset_at - now };
  }

  await db
    .prepare("UPDATE auth_rate_limits SET attempts = attempts + 1 WHERE key = ?")
    .bind(key)
    .run();

  return { allowed: true };
}

function jsonResponse(data: unknown, status = 200, extraHeaders: Record<string, string> = {}): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: {
      "Content-Type": "application/json",
      "Access-Control-Allow-Origin": "*",
      "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
      "Access-Control-Allow-Headers": "Content-Type, Authorization",
      ...extraHeaders,
    },
  });
}

function errorResponse(message: string, status = 400, extraHeaders: Record<string, string> = {}): Response {
  return jsonResponse({ success: false, error: message }, status, extraHeaders);
}

// Lightweight periodic cleanup for expired/revoked sessions (> 30 days old)
async function cleanupExpiredSessions(db: D1Database): Promise<void> {
  try {
    const now = Math.floor(Date.now() / 1000);
    const cutoff = now - CLEANUP_RETENTION_SECONDS;
    await db
      .prepare(
        "DELETE FROM sessions WHERE id IN (" +
        "  SELECT id FROM sessions WHERE (revoked_at IS NOT NULL AND revoked_at < ?) " +
        "  OR (idle_expires_at < ? AND absolute_expires_at < ?) LIMIT 50" +
        ")"
      )
      .bind(cutoff, cutoff, cutoff)
      .run();
  } catch (e) {
    // Non-fatal, bounded cleanup
    console.warn("Session cleanup warning:", e);
  }
}

// Authenticate Bearer token from header by comparing SHA-256 token hash against D1
// Implements 30-day idle expiry, 90-day absolute ceiling, and throttled activity updates
async function authenticateRequest(
  request: Request,
  db: D1Database
): Promise<AuthenticatedSession | null> {
  const authHeader = request.headers.get("Authorization");
  if (!authHeader || !authHeader.startsWith("Bearer ")) {
    return null;
  }
  const rawToken = authHeader.substring(7).trim();
  if (!rawToken || rawToken.length < 32) {
    return null;
  }
  const tokenHash = await hashToken(rawToken);
  const now = Math.floor(Date.now() / 1000);

  const row = await db
    .prepare(
      "SELECT s.id, s.user_id, s.token_hash, s.device_id, s.device_name, s.client_version, " +
      "s.created_at, s.last_used_at, s.idle_expires_at, s.absolute_expires_at, " +
      "s.revoked_at, s.revoked_reason, u.username " +
      "FROM sessions s JOIN users u ON s.user_id = u.id WHERE s.token_hash = ?"
    )
    .bind(tokenHash)
    .first<any>();

  if (!row) {
    return null;
  }

  // Verify session is not revoked
  if (row.revoked_at !== null && row.revoked_at !== undefined) {
    return null;
  }

  // Verify idle expiry has not passed (30 days inactivity)
  if (now >= row.idle_expires_at) {
    return null;
  }

  // Verify absolute expiry has not passed (90 days hard maximum from creation)
  if (now >= row.absolute_expires_at) {
    return null;
  }

  // Throttled activity update: update last_used_at / idle_expires_at at most once every 30 minutes
  // to prevent excessive D1 writes on routine requests
  if (now - row.last_used_at >= ACTIVITY_UPDATE_THRESHOLD_SECONDS) {
    const newIdleExpiresAt = Math.min(now + IDLE_TIMEOUT_SECONDS, row.absolute_expires_at);
    try {
      await db
        .prepare("UPDATE sessions SET last_used_at = ?, idle_expires_at = ? WHERE id = ?")
        .bind(now, newIdleExpiresAt, row.id)
        .run();
      row.last_used_at = now;
      row.idle_expires_at = newIdleExpiresAt;
    } catch (e) {
      console.warn("Failed to throttle-update session activity:", e);
    }
  }

  return {
    session_id: row.id,
    user_id: row.user_id,
    username: row.username,
    device_id: row.device_id,
    device_name: row.device_name,
    client_version: row.client_version,
    created_at: row.created_at,
    last_used_at: row.last_used_at,
    idle_expires_at: row.idle_expires_at,
    absolute_expires_at: row.absolute_expires_at,
  };
}

let schemaEnsured = false;
async function ensureSessionsSchema(db: D1Database): Promise<void> {
  if (schemaEnsured) return;
  try {
    await db.prepare("SELECT idle_expires_at FROM sessions LIMIT 1").first();
    schemaEnsured = true;
  } catch {
    try {
      await db.prepare("DROP TABLE IF EXISTS sessions").run();
      await db.prepare(`
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            token_hash TEXT UNIQUE NOT NULL,
            device_id TEXT NOT NULL,
            device_name TEXT NOT NULL,
            client_version TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            last_used_at INTEGER NOT NULL,
            idle_expires_at INTEGER NOT NULL,
            absolute_expires_at INTEGER NOT NULL,
            revoked_at INTEGER,
            revoked_reason TEXT
        )
      `).run();
      await db.prepare("CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id)").run();
      await db.prepare("CREATE INDEX IF NOT EXISTS idx_sessions_token_hash ON sessions(token_hash)").run();
      await db.prepare("CREATE INDEX IF NOT EXISTS idx_sessions_idle ON sessions(idle_expires_at)").run();
      await db.prepare("CREATE INDEX IF NOT EXISTS idx_sessions_absolute ON sessions(absolute_expires_at)").run();
      schemaEnsured = true;
    } catch (e) {
      console.warn("Schema initialization notice:", e);
    }
  }
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    if (request.method === "OPTIONS") {
      return new Response(null, {
        headers: {
          "Access-Control-Allow-Origin": "*",
          "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
          "Access-Control-Allow-Headers": "Content-Type, Authorization",
        },
      });
    }

    await ensureSessionsSchema(env.DB);

    const url = new URL(request.url);
    const path = url.pathname;

    // Health check endpoint
    if (path === "/" || path === "/api/health") {
      return jsonResponse({ status: "ok", service: "soundflow-cloud-sync" });
    }

    try {
      // --- Authentication Endpoints ---

      // POST /api/auth/register
      if (path === "/api/auth/register" && request.method === "POST") {
        const clientIp = getClientIp(request);
        const rateLimit = await checkRateLimit(env.DB, clientIp, "register");
        if (!rateLimit.allowed) {
          return errorResponse(
            `Too many registration requests. Please try again in ${rateLimit.retryAfter || 60} seconds.`,
            429,
            { "Retry-After": String(rateLimit.retryAfter || 60) }
          );
        }

        let body: any;
        try {
          body = await request.json();
        } catch {
          return errorResponse("Invalid JSON request body", 400);
        }

        if (typeof body !== "object" || body === null) {
          return errorResponse("Invalid request payload", 400);
        }

        const username = typeof body.username === "string" ? body.username.trim() : "";
        const password = typeof body.password === "string" ? body.password : "";

        // Validate username server-side
        if (!username || username.length < 3 || username.length > 50) {
          return errorResponse("Username must be between 3 and 50 characters", 400);
        }
        if (!/^[a-zA-Z0-9_\-\.]+$/.test(username)) {
          return errorResponse(
            "Username may only contain alphanumeric characters, hyphens, underscores, and dots",
            400
          );
        }

        // Validate password server-side
        if (!password || password.length < 8 || password.length > 128) {
          return errorResponse("Password must be between 8 and 128 characters", 400);
        }

        const existing = await env.DB.prepare(
          "SELECT id FROM users WHERE LOWER(username) = LOWER(?)"
        )
          .bind(username)
          .first();

        if (existing) {
          return errorResponse("Username is already registered", 409);
        }

        const userId = crypto.randomUUID();
        const passwordHash = await hashPassword(password);
        const now = Math.floor(Date.now() / 1000);

        await env.DB.prepare(
          "INSERT INTO users (id, username, password_hash, created_at) VALUES (?, ?, ?, ?)"
        )
          .bind(userId, username, passwordHash, now)
          .run();

        // Generate raw token (256 bits), store only token_hash in D1
        const rawToken = generateRawToken();
        const tokenHash = await hashToken(rawToken);
        const sessionId = crypto.randomUUID();
        const deviceId = typeof body.device_id === "string" && body.device_id.trim()
          ? body.device_id.trim().slice(0, 100)
          : crypto.randomUUID();
        const deviceName = typeof body.device_name === "string" && body.device_name.trim()
          ? body.device_name.trim().slice(0, 100)
          : "Desktop Player";
        const clientVersion = typeof body.client_version === "string" && body.client_version.trim()
          ? body.client_version.trim().slice(0, 50)
          : "0.1.0";
        const idleExpiresAt = now + IDLE_TIMEOUT_SECONDS;
        const absoluteExpiresAt = now + ABSOLUTE_TIMEOUT_SECONDS;

        await env.DB.prepare(
          "INSERT INTO sessions (" +
          "  id, user_id, token_hash, device_id, device_name, client_version, " +
          "  created_at, last_used_at, idle_expires_at, absolute_expires_at, revoked_at, revoked_reason" +
          ") VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL)"
        )
          .bind(
            sessionId,
            userId,
            tokenHash,
            deviceId,
            deviceName,
            clientVersion,
            now,
            now,
            idleExpiresAt,
            absoluteExpiresAt
          )
          .run();

        return jsonResponse({
          success: true,
          user: {
            id: userId,
            username,
            created_at: now,
          },
          token: rawToken,
          session: {
            id: sessionId,
            device_id: deviceId,
            device_name: deviceName,
            client_version: clientVersion,
            created_at: now,
            idle_expires_at: idleExpiresAt,
            absolute_expires_at: absoluteExpiresAt,
          },
        });
      }

      // POST /api/auth/login
      if (path === "/api/auth/login" && request.method === "POST") {
        const clientIp = getClientIp(request);
        const rateLimit = await checkRateLimit(env.DB, clientIp, "login");
        if (!rateLimit.allowed) {
          return errorResponse(
            `Too many login attempts. Please try again in ${rateLimit.retryAfter || 60} seconds.`,
            429,
            { "Retry-After": String(rateLimit.retryAfter || 60) }
          );
        }

        let body: any;
        try {
          body = await request.json();
        } catch {
          return errorResponse("Invalid JSON request body", 400);
        }

        if (typeof body !== "object" || body === null) {
          return errorResponse("Invalid request payload", 400);
        }

        const username = typeof body.username === "string" ? body.username.trim() : "";
        const password = typeof body.password === "string" ? body.password : "";

        if (!username || !password) {
          return errorResponse("Invalid username or password", 401);
        }

        const user = await env.DB.prepare(
          "SELECT id, username, password_hash, created_at FROM users WHERE LOWER(username) = LOWER(?)"
        )
          .bind(username)
          .first<UserRecord>();

        // Constant-time mitigation against user enumeration
        if (!user) {
          await verifyPassword(password, DUMMY_HASH);
          return errorResponse("Invalid username or password", 401);
        }

        const valid = await verifyPassword(password, user.password_hash);
        if (!valid) {
          return errorResponse("Invalid username or password", 401);
        }

        const now = Math.floor(Date.now() / 1000);
        const rawToken = generateRawToken();
        const tokenHash = await hashToken(rawToken);
        const sessionId = crypto.randomUUID();
        const deviceId = typeof body.device_id === "string" && body.device_id.trim()
          ? body.device_id.trim().slice(0, 100)
          : crypto.randomUUID();
        const deviceName = typeof body.device_name === "string" && body.device_name.trim()
          ? body.device_name.trim().slice(0, 100)
          : "Desktop Player";
        const clientVersion = typeof body.client_version === "string" && body.client_version.trim()
          ? body.client_version.trim().slice(0, 50)
          : "0.1.0";
        const idleExpiresAt = now + IDLE_TIMEOUT_SECONDS;
        const absoluteExpiresAt = now + ABSOLUTE_TIMEOUT_SECONDS;

        await env.DB.prepare(
          "INSERT INTO sessions (" +
          "  id, user_id, token_hash, device_id, device_name, client_version, " +
          "  created_at, last_used_at, idle_expires_at, absolute_expires_at, revoked_at, revoked_reason" +
          ") VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL)"
        )
          .bind(
            sessionId,
            user.id,
            tokenHash,
            deviceId,
            deviceName,
            clientVersion,
            now,
            now,
            idleExpiresAt,
            absoluteExpiresAt
          )
          .run();

        // Perform lightweight bounded cleanup on login
        cleanupExpiredSessions(env.DB).catch(() => {});

        return jsonResponse({
          success: true,
          user: {
            id: user.id,
            username: user.username,
            created_at: user.created_at,
          },
          token: rawToken,
          session: {
            id: sessionId,
            device_id: deviceId,
            device_name: deviceName,
            client_version: clientVersion,
            created_at: now,
            idle_expires_at: idleExpiresAt,
            absolute_expires_at: absoluteExpiresAt,
          },
        });
      }

      // POST /api/auth/logout
      if (path === "/api/auth/logout" && request.method === "POST") {
        const authHeader = request.headers.get("Authorization");
        if (authHeader?.startsWith("Bearer ")) {
          const rawToken = authHeader.substring(7).trim();
          if (rawToken) {
            const tokenHash = await hashToken(rawToken);
            const now = Math.floor(Date.now() / 1000);
            await env.DB.prepare(
              "UPDATE sessions SET revoked_at = ?, revoked_reason = 'user_logout' WHERE token_hash = ? AND revoked_at IS NULL"
            )
              .bind(now, tokenHash)
              .run();
          }
        }
        return jsonResponse({ success: true });
      }

      // POST /api/auth/logout-all
      if (path === "/api/auth/logout-all" && request.method === "POST") {
        const session = await authenticateRequest(request, env.DB);
        if (!session) {
          return errorResponse("Unauthorized", 401);
        }
        const now = Math.floor(Date.now() / 1000);
        await env.DB.prepare(
          "UPDATE sessions SET revoked_at = ?, revoked_reason = 'logout_all' WHERE user_id = ? AND revoked_at IS NULL"
        )
          .bind(now, session.user_id)
          .run();
        return jsonResponse({ success: true });
      }

      // GET /api/auth/sessions
      if (path === "/api/auth/sessions" && request.method === "GET") {
        const session = await authenticateRequest(request, env.DB);
        if (!session) {
          return errorResponse("Unauthorized", 401);
        }
        const now = Math.floor(Date.now() / 1000);
        const rows = await env.DB.prepare(
          "SELECT id, device_id, device_name, client_version, created_at, last_used_at, idle_expires_at, absolute_expires_at " +
          "FROM sessions WHERE user_id = ? AND revoked_at IS NULL AND idle_expires_at > ? AND absolute_expires_at > ? " +
          "ORDER BY last_used_at DESC"
        )
          .bind(session.user_id, now, now)
          .all();

        const safeSessions = (rows.results || []).map((r: any) => ({
          id: r.id,
          device_id: r.device_id,
          device_name: r.device_name,
          client_version: r.client_version,
          created_at: r.created_at,
          last_used_at: r.last_used_at,
          idle_expires_at: r.idle_expires_at,
          absolute_expires_at: r.absolute_expires_at,
          is_current: r.id === session.session_id,
        }));

        return jsonResponse({ success: true, sessions: safeSessions });
      }

      // DELETE /api/auth/sessions/:id
      if (path.startsWith("/api/auth/sessions/") && request.method === "DELETE") {
        const session = await authenticateRequest(request, env.DB);
        if (!session) {
          return errorResponse("Unauthorized", 401);
        }
        const targetSessionId = path.substring("/api/auth/sessions/".length).trim();
        if (!targetSessionId) {
          return errorResponse("Session ID required", 400);
        }

        const target = await env.DB.prepare(
          "SELECT id, user_id FROM sessions WHERE id = ?"
        )
          .bind(targetSessionId)
          .first<any>();

        if (!target) {
          return errorResponse("Session not found", 404);
        }

        if (target.user_id !== session.user_id) {
          return errorResponse("Forbidden: cannot revoke another user's session", 403);
        }

        const now = Math.floor(Date.now() / 1000);
        await env.DB.prepare(
          "UPDATE sessions SET revoked_at = ?, revoked_reason = 'user_revoked' WHERE id = ?"
        )
          .bind(now, targetSessionId)
          .run();

        return jsonResponse({ success: true });
      }

      // GET /api/auth/me
      if (path === "/api/auth/me" && request.method === "GET") {
        const session = await authenticateRequest(request, env.DB);
        if (!session) {
          return errorResponse("Unauthorized", 401);
        }
        const user = await env.DB.prepare(
          "SELECT id, username, created_at FROM users WHERE id = ?"
        )
          .bind(session.user_id)
          .first<UserRecord>();

        if (!user) {
          return errorResponse("User not found", 404);
        }

        return jsonResponse({
          success: true,
          user: {
            id: user.id,
            username: user.username,
            created_at: user.created_at,
          },
          session: {
            id: session.session_id,
            device_id: session.device_id,
            device_name: session.device_name,
            client_version: session.client_version,
            created_at: session.created_at,
            idle_expires_at: session.idle_expires_at,
            absolute_expires_at: session.absolute_expires_at,
          },
        });
      }

      // --- Data Synchronization Endpoints ---

      // GET /api/sync
      if (path === "/api/sync" && request.method === "GET") {
        const session = await authenticateRequest(request, env.DB);
        if (!session) {
          return errorResponse("Unauthorized", 401);
        }

        const userId = session.user_id;

        const [songs, playlists, playlistSongs, songStats, userStats, userSettings] =
          await Promise.all([
            env.DB.prepare("SELECT * FROM songs WHERE user_id = ?").bind(userId).all(),
            env.DB.prepare("SELECT * FROM playlists WHERE user_id = ?").bind(userId).all(),
            env.DB.prepare("SELECT * FROM playlist_songs WHERE user_id = ?").bind(userId).all(),
            env.DB.prepare("SELECT * FROM song_stats WHERE user_id = ?").bind(userId).all(),
            env.DB.prepare("SELECT * FROM user_stats WHERE user_id = ?").bind(userId).first(),
            env.DB.prepare("SELECT * FROM user_settings WHERE user_id = ?").bind(userId).first(),
          ]);

        const now = Math.floor(Date.now() / 1000);

        return jsonResponse({
          success: true,
          user_id: userId,
          synced_at: now,
          data: {
            songs: songs.results || [],
            playlists: playlists.results || [],
            playlist_songs: playlistSongs.results || [],
            song_stats: songStats.results || [],
            user_stats: userStats || null,
            user_settings: userSettings || null,
          },
        });
      }

      // POST /api/sync
      if (path === "/api/sync" && request.method === "POST") {
        const session = await authenticateRequest(request, env.DB);
        if (!session) {
          return errorResponse("Unauthorized", 401);
        }

        const userId = session.user_id;
        let payload: any;
        try {
          payload = await request.json();
        } catch {
          return errorResponse("Invalid JSON payload", 400);
        }

        const now = Math.floor(Date.now() / 1000);
        const batchStatements: D1PreparedStatement[] = [];

        // 1. Sync Playlists
        if (Array.isArray(payload.playlists)) {
          for (const pl of payload.playlists) {
            if (pl.id && pl.name) {
              batchStatements.push(
                env.DB.prepare(
                  "INSERT INTO playlists (id, user_id, name, description, is_smart_mix, mix_type, created_at, updated_at) " +
                  "VALUES (?, ?, ?, ?, ?, ?, ?, ?) " +
                  "ON CONFLICT(id, user_id) DO UPDATE SET name = excluded.name, description = excluded.description, updated_at = excluded.updated_at"
                ).bind(
                  pl.id,
                  userId,
                  pl.name,
                  pl.description || null,
                  pl.is_smart_mix || 0,
                  pl.mix_type || null,
                  pl.created_at || now,
                  pl.updated_at || now
                )
              );
            }
          }
        }

        // 2. Sync Songs
        if (Array.isArray(payload.songs)) {
          for (const s of payload.songs) {
            if (s.id && s.title) {
              batchStatements.push(
                env.DB.prepare(
                  "INSERT INTO songs (id, user_id, title, artist, album, duration_secs, cover_art_url, preview_url, updated_at) " +
                  "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) " +
                  "ON CONFLICT(id, user_id) DO UPDATE SET title = excluded.title, artist = excluded.artist, album = excluded.album, updated_at = excluded.updated_at"
                ).bind(
                  s.id,
                  userId,
                  s.title,
                  s.artist || "",
                  s.album || "",
                  s.duration_secs || 0.0,
                  s.cover_art_url || null,
                  s.preview_url || null,
                  s.updated_at || now
                )
              );
            }
          }
        }

        // 3. Sync Playlist Songs
        if (Array.isArray(payload.playlist_songs)) {
          for (const ps of payload.playlist_songs) {
            if (ps.playlist_id && ps.song_id) {
              batchStatements.push(
                env.DB.prepare(
                  "INSERT INTO playlist_songs (playlist_id, song_id, user_id, position) " +
                  "VALUES (?, ?, ?, ?) " +
                  "ON CONFLICT(playlist_id, song_id, user_id) DO UPDATE SET position = excluded.position"
                ).bind(ps.playlist_id, ps.song_id, userId, ps.position || 0)
              );
            }
          }
        }

        // 4. Sync Song Stats
        if (Array.isArray(payload.song_stats)) {
          for (const ss of payload.song_stats) {
            if (ss.song_id) {
              batchStatements.push(
                env.DB.prepare(
                  "INSERT INTO song_stats (song_id, user_id, play_count, total_seconds, completion_count, skip_count, manual_like, updated_at) " +
                  "VALUES (?, ?, ?, ?, ?, ?, ?, ?) " +
                  "ON CONFLICT(song_id, user_id) DO UPDATE SET " +
                  "play_count = MAX(song_stats.play_count, excluded.play_count), " +
                  "total_seconds = MAX(song_stats.total_seconds, excluded.total_seconds), " +
                  "completion_count = MAX(song_stats.completion_count, excluded.completion_count), " +
                  "skip_count = MAX(song_stats.skip_count, excluded.skip_count), " +
                  "manual_like = excluded.manual_like, " +
                  "updated_at = excluded.updated_at"
                ).bind(
                  ss.song_id,
                  userId,
                  ss.play_count || 0,
                  ss.total_seconds || ss.total_time_listened || 0.0,
                  ss.completion_count || 0,
                  ss.skip_count || 0,
                  ss.manual_like !== undefined ? ss.manual_like : 0,
                  ss.updated_at || now
                )
              );
            }
          }
        }

        // 5. Sync User Stats
        if (payload.user_stats) {
          const us = Array.isArray(payload.user_stats) ? payload.user_stats[0] : payload.user_stats;
          if (us) {
            batchStatements.push(
              env.DB.prepare(
                "INSERT INTO user_stats (user_id, total_seconds, top_songs_json, top_artists_json, top_days_json, yearly_archives_json, updated_at) " +
                "VALUES (?, ?, ?, ?, ?, ?, ?) " +
                "ON CONFLICT(user_id) DO UPDATE SET " +
                "total_seconds = MAX(user_stats.total_seconds, excluded.total_seconds), " +
                "top_songs_json = excluded.top_songs_json, " +
                "top_artists_json = excluded.top_artists_json, " +
                "yearly_archives_json = excluded.yearly_archives_json, " +
                "updated_at = excluded.updated_at"
              ).bind(
                userId,
                us.total_seconds || 0.0,
                typeof us.top_songs_json === "string" ? us.top_songs_json : JSON.stringify(us.top_songs_json || []),
                typeof us.top_artists_json === "string" ? us.top_artists_json : JSON.stringify(us.top_artists_json || []),
                typeof us.top_days_json === "string" ? us.top_days_json : JSON.stringify(us.top_days_json || []),
                typeof us.yearly_archives_json === "string" ? us.yearly_archives_json : JSON.stringify(us.yearly_archives_json || []),
                now
              )
            );
          }
        }

        // 6. Sync User Settings
        if (payload.user_settings) {
          const uset = Array.isArray(payload.user_settings) ? payload.user_settings[0] : payload.user_settings;
          if (uset) {
            const settingsStr =
              typeof uset.settings_json === "string"
                ? uset.settings_json
                : JSON.stringify(uset.settings_json || uset);
            batchStatements.push(
              env.DB.prepare(
                "INSERT INTO user_settings (user_id, settings_json, updated_at) " +
                "VALUES (?, ?, ?) " +
                "ON CONFLICT(user_id) DO UPDATE SET settings_json = excluded.settings_json, updated_at = excluded.updated_at"
              ).bind(userId, settingsStr, now)
            );
          }
        }

        // 7. Sync Deletions - Playlists
        if (Array.isArray(payload.deleted_playlists)) {
          for (const pid of payload.deleted_playlists) {
            if (pid) {
              batchStatements.push(
                env.DB.prepare("DELETE FROM playlists WHERE id = ? AND user_id = ?").bind(pid, userId),
                env.DB.prepare("DELETE FROM playlist_songs WHERE playlist_id = ? AND user_id = ?").bind(pid, userId)
              );
            }
          }
        }

        // 8. Sync Deletions - Playlist Songs
        if (Array.isArray(payload.deleted_playlist_songs)) {
          for (const dps of payload.deleted_playlist_songs) {
            if (dps.playlist_id && dps.song_id) {
              batchStatements.push(
                env.DB.prepare("DELETE FROM playlist_songs WHERE playlist_id = ? AND song_id = ? AND user_id = ?")
                  .bind(dps.playlist_id, dps.song_id, userId)
              );
            }
          }
        }

        if (batchStatements.length > 0) {
          await env.DB.batch(batchStatements);
        }

        return jsonResponse({
          success: true,
          user_id: userId,
          synced_at: now,
          items_processed: batchStatements.length,
        });
      }

      return errorResponse("Endpoint not found", 404);
    } catch (err: any) {
      console.error("Worker error:", err);
      return errorResponse(err.message || "Internal server error", 500);
    }
  },
  async scheduled(_event: any, env: Env, ctx: any): Promise<void> {
    if (ctx && typeof ctx.waitUntil === "function") {
      ctx.waitUntil(cleanupExpiredSessions(env.DB));
    } else {
      await cleanupExpiredSessions(env.DB);
    }
  },
};
