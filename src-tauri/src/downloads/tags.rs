use super::DownloadTags;
use crate::core::{AppError, AppResult};
use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::config::ParseOptions;
use lofty::mp4::{Atom, AtomData, AtomIdent, Ilst, Mp4File};
use lofty::tag::{Accessor, Tag};
use std::path::Path;

/// Freeform MP4 atom (`----:com.kaze:jiosaavn_id`) holding the JioSaavn recording id.
fn source_atom() -> AtomIdent<'static> {
    AtomIdent::Freeform {
        mean: "com.kaze".into(),
        name: "jiosaavn_id".into(),
    }
}

fn read_mp4(path: &Path) -> Option<Mp4File> {
    let mut file = std::fs::File::open(path).ok()?;
    Mp4File::read_from(&mut file, ParseOptions::new()).ok()
}

/// The JioSaavn id Kaze embedded in this file, if any (MP4/M4A only).
pub fn read_source_id(path: &Path) -> Option<String> {
    let mp4 = read_mp4(path)?;
    let atom = mp4.ilst()?.get(&source_atom())?;
    let id = atom.data().find_map(|data| match data {
        AtomData::UTF8(id) if !id.trim().is_empty() => Some(id.clone()),
        _ => None,
    });
    id
}

/// Stores the recording id in the file. Other players ignore this atom.
fn write_source_id(path: &Path, id: &str) -> AppResult<()> {
    let mut mp4 = read_mp4(path).ok_or_else(|| AppError::Metadata("Not an MP4 file".into()))?;
    if mp4.ilst().is_none() {
        mp4.set_ilst(Ilst::new());
    }
    if let Some(ilst) = mp4.ilst_mut() {
        ilst.replace_atom(Atom::new(source_atom(), AtomData::UTF8(id.to_string())));
    }
    mp4.save_to_path(path, WriteOptions::default())
        .map_err(|e| AppError::Metadata(e.to_string()))
}

/// Embeds display metadata so the library shows the song, not its filename.
pub fn write_tags(path: &Path, tags: &DownloadTags) -> AppResult<()> {
    let err = |e: lofty::error::LoftyError| AppError::Metadata(e.to_string());
    let mut file = Probe::open(path)
        .map_err(err)?
        .guess_file_type()
        .map_err(|e| AppError::Metadata(e.to_string()))?
        .read()
        .map_err(err)?;
    let tag_type = file.primary_tag_type();
    if file.primary_tag().is_none() {
        file.insert_tag(Tag::new(tag_type));
    }
    let tag = file
        .primary_tag_mut()
        .ok_or_else(|| AppError::Metadata("No writable tag".into()))?;
    tag.set_title(tags.title.clone());
    tag.set_artist(tags.artist.clone());
    if let Some(album) = tags.album.as_ref().filter(|a| !a.trim().is_empty()) {
        tag.set_album(album.clone());
    }
    if let Some(year) = tags.year {
        tag.set_year(year);
    }
    file.save_to_path(path, WriteOptions::default()).map_err(err)?;
    if let Some(id) = tags.source_id.as_ref().filter(|id| !id.is_empty()) {
        write_source_id(path, id)?;
    }
    Ok(())
}
