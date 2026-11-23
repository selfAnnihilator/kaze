//! Gives a finished download its final name without ever overwriting a file
//! saved there meanwhile.

use std::path::Path;
use tokio::io::AsyncWriteExt;

/// Publishes `partial` as `destination`. A hard link is atomic and keeps the
/// original, but Android's shared storage (FUSE) refuses hard links, so fall
/// back to copying into a file created with `create_new`.
pub(super) async fn publish(partial: &Path, destination: &Path) -> std::io::Result<()> {
    match tokio::fs::hard_link(partial, destination).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Err(e),
        Err(_) => copy_new(partial, destination).await,
    }
}

/// Copies `from` into a brand-new `to`, synced to disk; a half-written `to` is removed.
pub(super) async fn copy_new(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut target = tokio::fs::OpenOptions::new().write(true).create_new(true).open(to).await?;
    let copied = async {
        let mut source = tokio::fs::File::open(from).await?;
        tokio::io::copy(&mut source, &mut target).await?;
        target.flush().await?;
        target.sync_all().await
    }
    .await;
    if copied.is_err() {
        drop(target);
        let _ = tokio::fs::remove_file(to).await;
    }
    copied
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("kaze-publish-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[tokio::test]
    async fn copy_fallback_writes_the_whole_file() {
        let d = dir();
        let (from, to) = (d.join("a.part"), d.join("a.m4a"));
        std::fs::write(&from, b"audio bytes").unwrap();
        copy_new(&from, &to).await.unwrap();
        assert_eq!(std::fs::read(&to).unwrap(), b"audio bytes");
        std::fs::remove_dir_all(d).unwrap();
    }

    #[tokio::test]
    async fn never_overwrites_an_existing_file() {
        let d = dir();
        let (from, to) = (d.join("b.part"), d.join("b.m4a"));
        std::fs::write(&from, b"new").unwrap();
        std::fs::write(&to, b"kept").unwrap();
        assert!(publish(&from, &to).await.is_err());
        assert!(copy_new(&from, &to).await.is_err());
        assert_eq!(std::fs::read(&to).unwrap(), b"kept");
        std::fs::remove_dir_all(d).unwrap();
    }

    #[tokio::test]
    async fn publish_links_when_it_can() {
        let d = dir();
        let (from, to) = (d.join("c.part"), d.join("c.m4a"));
        std::fs::write(&from, b"linked").unwrap();
        publish(&from, &to).await.unwrap();
        assert_eq!(std::fs::read(&to).unwrap(), b"linked");
        std::fs::remove_dir_all(d).unwrap();
    }
}
