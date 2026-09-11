pub mod coordinator;
pub mod cover_art_archive;
pub mod musicbrainz;
pub mod spotify;
pub mod types;

pub use coordinator::*;
pub use cover_art_archive::*;
pub use musicbrainz::*;
pub use spotify::*;
pub use types::*;

use crate::core::error::AppResult;
use async_trait::async_trait;

#[async_trait]
pub trait MetadataProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn is_available(&self) -> bool;

    async fn search_track(
        &self,
        title: &str,
        artist: &str,
        album: Option<&str>,
    ) -> AppResult<Vec<ExternalTrackMetadata>>;

    async fn search_artist(&self, name: &str) -> AppResult<Option<ExternalArtistMetadata>>;
    async fn search_album(&self, title: &str, artist: &str) -> AppResult<Option<ExternalAlbumMetadata>>;
    async fn fetch_cover_art(&self, release_id: &str) -> AppResult<Option<Vec<u8>>>;
}
