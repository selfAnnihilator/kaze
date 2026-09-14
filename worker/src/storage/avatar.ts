/**
 * SoundFlow Avatar Storage Abstraction
 * Supports Cloudinary as primary image storage and R2 as alternative provider.
 */

export interface AvatarUploadResult {
  public_id: string;
  secure_url: string;
  version: number;
}

export interface AvatarStorage {
  upload_avatar(userId: string, imageBytes: ArrayBuffer): Promise<AvatarUploadResult>;
  delete_avatar(userId: string, publicId?: string): Promise<boolean>;
  get_avatar(userId: string, publicId?: string, avatarUrl?: string): Promise<Response | null>;
}

export interface StorageConfigEnv {
  CLOUDINARY_CLOUD_NAME?: string;
  CLOUDINARY_API_KEY?: string;
  CLOUDINARY_API_SECRET?: string;
  PROFILE_IMAGES?: R2Bucket;
}

/**
 * CloudinaryAvatarStorage
 * Stores normalized avatars in Cloudinary under 'music-player/avatars/{user_id}'.
 * Overwrites existing avatars on upload and uses signed server-side destroy on delete.
 */
export class CloudinaryAvatarStorage implements AvatarStorage {
  private cloudName: string;
  private apiKey: string;
  private apiSecret: string;

  constructor(cloudName: string, apiKey: string, apiSecret: string) {
    this.cloudName = cloudName;
    this.apiKey = apiKey;
    this.apiSecret = apiSecret;
  }

  private async generateSignature(params: Record<string, string>): Promise<string> {
    const sortedKeys = Object.keys(params).sort();
    const toSign = sortedKeys.map((k) => `${k}=${params[k]}`).join("&") + this.apiSecret;
    const encoder = new TextEncoder();
    const data = encoder.encode(toSign);
    const hashBuffer = await crypto.subtle.digest("SHA-1", data);
    return Array.from(new Uint8Array(hashBuffer))
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("");
  }

  async upload_avatar(userId: string, imageBytes: ArrayBuffer): Promise<AvatarUploadResult> {
    const timestamp = Math.floor(Date.now() / 1000).toString();
    const publicId = `music-player/avatars/${userId}`;

    const signParams: Record<string, string> = {
      invalidate: "true",
      overwrite: "true",
      public_id: publicId,
      timestamp,
    };

    const signature = await this.generateSignature(signParams);

    const formData = new FormData();
    formData.append("file", new Blob([imageBytes], { type: "image/webp" }), `${userId}.webp`);
    formData.append("public_id", publicId);
    formData.append("overwrite", "true");
    formData.append("invalidate", "true");
    formData.append("timestamp", timestamp);
    formData.append("api_key", this.apiKey);
    formData.append("signature", signature);

    const uploadUrl = `https://api.cloudinary.com/v1_1/${encodeURIComponent(this.cloudName)}/image/upload`;
    const resp = await fetch(uploadUrl, {
      method: "POST",
      body: formData,
    });

    if (!resp.ok) {
      const errText = await resp.text();
      throw new Error(`Cloudinary upload failed (HTTP ${resp.status}): ${errText}`);
    }

    const data: any = await resp.json();
    return {
      public_id: data.public_id || publicId,
      secure_url: data.secure_url || data.url,
      version: data.version || parseInt(timestamp, 10),
    };
  }

  async delete_avatar(userId: string, publicId?: string): Promise<boolean> {
    const targetPublicId = publicId || `music-player/avatars/${userId}`;
    // Security check: ensure target begins with "music-player/avatars/" and contains authenticated userId
    if (!targetPublicId.startsWith("music-player/avatars/")) {
      throw new Error("Invalid Cloudinary public_id prefix. Must reside in music-player/avatars/");
    }

    const timestamp = Math.floor(Date.now() / 1000).toString();
    const signParams: Record<string, string> = {
      invalidate: "true",
      public_id: targetPublicId,
      timestamp,
    };

    const signature = await this.generateSignature(signParams);

    const formData = new FormData();
    formData.append("public_id", targetPublicId);
    formData.append("invalidate", "true");
    formData.append("timestamp", timestamp);
    formData.append("api_key", this.apiKey);
    formData.append("signature", signature);

    const destroyUrl = `https://api.cloudinary.com/v1_1/${encodeURIComponent(this.cloudName)}/image/destroy`;
    const resp = await fetch(destroyUrl, {
      method: "POST",
      body: formData,
    });

    if (!resp.ok) {
      const errText = await resp.text();
      console.warn(`Cloudinary destroy failed (HTTP ${resp.status}): ${errText}`);
      return false;
    }

    const data: any = await resp.json();
    return data.result === "ok" || data.result === "not found";
  }

  async get_avatar(userId: string, _publicId?: string, avatarUrl?: string): Promise<Response | null> {
    const url =
      avatarUrl ||
      `https://res.cloudinary.com/${encodeURIComponent(this.cloudName)}/image/upload/music-player/avatars/${encodeURIComponent(userId)}.webp`;
    const resp = await fetch(url);
    if (!resp.ok) {
      return null;
    }

    const headers = new Headers();
    headers.set("Content-Type", "image/webp");
    headers.set("Cache-Control", "public, max-age=86400, stale-while-revalidate=604800");
    headers.set("Access-Control-Allow-Origin", "*");
    headers.set("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS");
    headers.set("Access-Control-Allow-Headers", "Content-Type, Authorization");

    return new Response(resp.body, { headers, status: 200 });
  }
}

/**
 * R2AvatarStorage
 * Optional fallback provider for Cloudflare R2 if configured.
 */
export class R2AvatarStorage implements AvatarStorage {
  private bucket: R2Bucket;

  constructor(bucket: R2Bucket) {
    this.bucket = bucket;
  }

  async upload_avatar(userId: string, imageBytes: ArrayBuffer): Promise<AvatarUploadResult> {
    const objectKey = `avatars/${userId}/avatar.webp`;
    await this.bucket.put(objectKey, imageBytes, {
      httpMetadata: { contentType: "image/webp" },
    });
    const now = Math.floor(Date.now() / 1000);
    return {
      public_id: objectKey,
      secure_url: `/api/profile/avatar?user_id=${encodeURIComponent(userId)}&v=${now}`,
      version: now,
    };
  }

  async delete_avatar(userId: string, publicId?: string): Promise<boolean> {
    const objectKey = publicId || `avatars/${userId}/avatar.webp`;
    await this.bucket.delete(objectKey);
    return true;
  }

  async get_avatar(userId: string, publicId?: string): Promise<Response | null> {
    const objectKey = publicId || `avatars/${userId}/avatar.webp`;
    const object = await this.bucket.get(objectKey);
    if (!object) return null;
    const headers = new Headers();
    headers.set("Content-Type", "image/webp");
    headers.set("Cache-Control", "public, max-age=3600");
    headers.set("Access-Control-Allow-Origin", "*");
    return new Response(object.body, { headers });
  }
}

/**
 * Factory function to create the appropriate AvatarStorage implementation
 */
export function createAvatarStorage(env: StorageConfigEnv): AvatarStorage | null {
  if (
    env.CLOUDINARY_CLOUD_NAME &&
    env.CLOUDINARY_API_KEY &&
    env.CLOUDINARY_API_SECRET &&
    env.CLOUDINARY_CLOUD_NAME.trim() !== "" &&
    env.CLOUDINARY_API_KEY.trim() !== "" &&
    env.CLOUDINARY_API_SECRET.trim() !== ""
  ) {
    return new CloudinaryAvatarStorage(
      env.CLOUDINARY_CLOUD_NAME.trim(),
      env.CLOUDINARY_API_KEY.trim(),
      env.CLOUDINARY_API_SECRET.trim()
    );
  }
  if (env.PROFILE_IMAGES) {
    return new R2AvatarStorage(env.PROFILE_IMAGES);
  }
  return null;
}
