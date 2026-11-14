use std::path::Path;

/// Which part of the library a listing covers. Kaze's own downloads live in one
/// folder; everything else is the user's local library. Derived from the file
/// path, so no schema change is needed (older app versions keep working).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum LibraryScope {
    #[default]
    All,
    /// The user's own folders: everything outside the Kaze folder.
    Local { downloads_prefix: String },
    /// Songs Kaze downloaded into its folder.
    Downloads { downloads_prefix: String },
}

impl LibraryScope {
    /// `scope` is the IPC value ("local" / "downloads"); anything else means all.
    pub fn parse(scope: Option<&str>, downloads_dir: &Path) -> Self {
        let dir = downloads_dir
            .canonicalize()
            .unwrap_or_else(|_| downloads_dir.to_path_buf());
        let mut downloads_prefix = dir.to_string_lossy().trim_end_matches('/').to_string();
        downloads_prefix.push(std::path::MAIN_SEPARATOR);
        match scope {
            Some("local") => Self::Local { downloads_prefix },
            Some("downloads") => Self::Downloads { downloads_prefix },
            _ => Self::All,
        }
    }

    /// An SQL condition on `column` (prefix compare, not LIKE: paths may hold `_`
    /// or `%`) and its two binds, or `None` for all songs.
    pub fn condition(&self, column: &str) -> Option<(String, i64, String)> {
        let (op, prefix) = match self {
            Self::All => return None,
            Self::Local { downloads_prefix } => ("!=", downloads_prefix),
            Self::Downloads { downloads_prefix } => ("=", downloads_prefix),
        };
        Some((
            format!("substr({column}, 1, ?) {op} ?"),
            prefix.chars().count() as i64,
            prefix.clone(),
        ))
    }
}
