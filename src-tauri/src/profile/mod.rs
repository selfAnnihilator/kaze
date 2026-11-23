use crate::core::error::{AppError, AppResult};
use image::{imageops::FilterType, GenericImageView};
use std::io::Cursor;
use std::path::PathBuf;

pub struct ProfileService {
    cache_dir: PathBuf,
}

/// Largest avatar image accepted before normalising (matches `normalize_avatar_image`).
pub const MAX_AVATAR_UPLOAD_BYTES: usize = 5 * 1024 * 1024;

impl ProfileService {
    pub fn new(cache_dir: PathBuf) -> Self {
        let avatars_dir = cache_dir.join("avatars");
        let _ = std::fs::create_dir_all(&avatars_dir);
        Self { cache_dir }
    }

    pub fn avatars_dir(&self) -> PathBuf {
        self.cache_dir.join("avatars")
    }

    pub fn get_cached_avatar_path(&self, user_id: &str) -> PathBuf {
        self.avatars_dir().join(format!("{}.webp", user_id))
    }

    pub fn has_cached_avatar(&self, user_id: &str) -> bool {
        self.get_cached_avatar_path(user_id).exists()
    }

    pub fn get_cached_avatar_data_url(&self, user_id: &str) -> Option<String> {
        let path = self.get_cached_avatar_path(user_id);
        if !path.exists() {
            return None;
        }
        let bytes = std::fs::read(&path).ok()?;
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        Some(format!("data:image/webp;base64,{}", b64))
    }

    pub fn save_cached_avatar(&self, user_id: &str, bytes: &[u8]) -> AppResult<PathBuf> {
        let path = self.get_cached_avatar_path(user_id);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&path, bytes).map_err(|e| AppError::Io(e.to_string()))?;
        Ok(path)
    }

    pub fn remove_cached_avatar(&self, user_id: &str) {
        let path = self.get_cached_avatar_path(user_id);
        let _ = std::fs::remove_file(path);
    }

    /// Validates, square-crops (centered), resizes to 256x256, and encodes as WebP
    pub fn normalize_avatar_image(raw_bytes: &[u8]) -> AppResult<Vec<u8>> {
        if raw_bytes.is_empty() {
            return Err(AppError::Validation("Image file is empty (zero bytes)".to_string()));
        }
        if raw_bytes.len() > MAX_AVATAR_UPLOAD_BYTES {
            return Err(AppError::Validation("Image exceeds maximum 5 MB limit".to_string()));
        }

        // Decode image from bytes (accepts JPEG, PNG, WebP)
        let img = image::load_from_memory(raw_bytes)
            .map_err(|e| AppError::Validation(format!("Unsupported or malformed image: {}", e)))?;

        let (width, height) = img.dimensions();
        if width == 0 || height == 0 {
            return Err(AppError::Validation("Image dimensions are zero".to_string()));
        }

        // Center square crop
        let square_size = width.min(height);
        let x = (width - square_size) / 2;
        let y = (height - square_size) / 2;
        let cropped = img.crop_imm(x, y, square_size, square_size);

        // Resize to 256x256
        let resized = cropped.resize_exact(256, 256, FilterType::Lanczos3);

        // Encode to WebP
        let mut webp_bytes = Vec::new();
        resized
            .write_to(&mut Cursor::new(&mut webp_bytes), image::ImageFormat::WebP)
            .map_err(|e| AppError::Validation(format!("Failed to encode WebP image: {}", e)))?;

        Ok(webp_bytes)
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    pub fn pick_avatar_file() -> AppResult<Option<PathBuf>> {
        let file = rfd::FileDialog::new()
            .set_title("Select Profile Photo")
            .add_filter("Images (*.jpg, *.jpeg, *.png, *.webp)", &["jpg", "jpeg", "png", "webp"])
            .pick_file();
        Ok(file)
    }

    /// The picked photo's bytes: sent inline (phone), from a path, or from the
    /// native dialog. `None` when the dialog was cancelled.
    pub fn avatar_upload_bytes(file_path: Option<String>, image_base64: Option<String>) -> AppResult<Option<Vec<u8>>> {
        if let Some(encoded) = image_base64.filter(|b| !b.trim().is_empty()) {
            return Self::decode_avatar_upload(&encoded).map(Some);
        }
        let path = match file_path.filter(|p| !p.trim().is_empty()) {
            Some(p) => PathBuf::from(p),
            None => match Self::pick_avatar_file()? {
                Some(p) => p,
                None => return Ok(None),
            },
        };
        std::fs::read(&path)
            .map(Some)
            .map_err(|e| AppError::Io(format!("Failed to read selected image: {}", e)))
    }

    /// Decodes an image sent as base64 (optionally a `data:` URL) by the phone's
    /// own picker. Rejects anything over `MAX_AVATAR_UPLOAD_BYTES`.
    pub fn decode_avatar_upload(encoded: &str) -> AppResult<Vec<u8>> {
        use base64::Engine;
        let body = encoded.split_once(";base64,").map_or(encoded, |(_, b)| b).trim();
        if body.len() / 4 * 3 > MAX_AVATAR_UPLOAD_BYTES {
            return Err(AppError::Validation("That photo is too large. Pick one under 5 MB.".into()));
        }
        base64::engine::general_purpose::STANDARD
            .decode(body)
            .map_err(|_| AppError::Validation("Couldn’t read that photo. Pick another image.".into()))
    }

    /// Mobile has no native file dialog yet; the picker bridge lands with the
    /// kaze-media plugin.
    #[cfg(any(target_os = "android", target_os = "ios"))]
    pub fn pick_avatar_file() -> AppResult<Option<PathBuf>> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    #[test]
    fn test_normalize_avatar_image_and_cache() {
        // Create an in-memory 500x300 RGBA image
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(500, 300, Rgba([255, 100, 50, 255]));
        let mut raw_png = Vec::new();
        img.write_to(&mut Cursor::new(&mut raw_png), image::ImageFormat::Png).expect("png encode");

        // Normalize to 256x256 WebP
        let webp_bytes = ProfileService::normalize_avatar_image(&raw_png).expect("normalize avatar ok");
        assert!(!webp_bytes.is_empty());

        // Verify decoded normalized dimensions are 256x256
        let decoded = image::load_from_memory(&webp_bytes).expect("decode webp");
        assert_eq!(decoded.dimensions(), (256, 256));

        // Test caching
        let temp_dir = std::env::temp_dir().join(format!("soundflow_profile_test_{}", uuid::Uuid::new_v4()));
        let service = ProfileService::new(temp_dir.clone());
        let user_id = "test_user_avatar_1";

        assert!(!service.has_cached_avatar(user_id));
        assert!(service.get_cached_avatar_data_url(user_id).is_none());

        service.save_cached_avatar(user_id, &webp_bytes).expect("save cached avatar");
        assert!(service.has_cached_avatar(user_id));

        let data_url = service.get_cached_avatar_data_url(user_id).expect("get cached avatar data url");
        assert!(data_url.starts_with("data:image/webp;base64,"));

        service.remove_cached_avatar(user_id);
        assert!(!service.has_cached_avatar(user_id));
        assert!(service.get_cached_avatar_data_url(user_id).is_none());

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_normalize_avatar_empty_and_oversized() {
        let empty = Vec::new();
        assert!(ProfileService::normalize_avatar_image(&empty).is_err());

        let oversized = vec![0u8; 6 * 1024 * 1024]; // 6 MB exceeds 5 MB limit
        assert!(ProfileService::normalize_avatar_image(&oversized).is_err());
    }

    #[test]
    fn test_decode_avatar_upload_accepts_data_urls_and_rejects_junk() {
        use base64::Engine;
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(40, 30, Rgba([1, 2, 3, 255]));
        let mut raw_png = Vec::new();
        img.write_to(&mut Cursor::new(&mut raw_png), image::ImageFormat::Png).expect("png encode");
        let b64 = base64::engine::general_purpose::STANDARD.encode(&raw_png);

        let plain = ProfileService::decode_avatar_upload(&b64).expect("plain base64");
        let url = ProfileService::decode_avatar_upload(&format!("data:image/png;base64,{b64}")).expect("data url");
        assert_eq!(plain, raw_png);
        assert_eq!(url, raw_png);
        assert!(!ProfileService::normalize_avatar_image(&url).expect("normalizes").is_empty());

        assert!(ProfileService::decode_avatar_upload("not base64 !!").is_err());
        let huge = "A".repeat(MAX_AVATAR_UPLOAD_BYTES / 3 * 4 + 8);
        assert!(ProfileService::decode_avatar_upload(&huge).is_err());
    }
}
