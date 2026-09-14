use crate::core::error::{AppError, AppResult};
use image::{imageops::FilterType, GenericImageView};
use std::io::Cursor;
use std::path::PathBuf;

pub struct ProfileService {
    cache_dir: PathBuf,
}

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
        if raw_bytes.len() > 5 * 1024 * 1024 {
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

    pub fn pick_avatar_file() -> AppResult<Option<PathBuf>> {
        let file = rfd::FileDialog::new()
            .set_title("Select Profile Photo")
            .add_filter("Images (*.jpg, *.jpeg, *.png, *.webp)", &["jpg", "jpeg", "png", "webp"])
            .pick_file();
        Ok(file)
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
}
