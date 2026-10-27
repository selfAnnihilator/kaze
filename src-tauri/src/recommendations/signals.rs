//! Account-scoped song identity and feedback across saved aliases/provider releases.
use crate::core::AppResult;
use sqlx::{FromRow, SqlitePool};
use std::collections::{HashMap, HashSet};

pub fn artist_credits(artist: &str) -> Vec<String> {
    let clean = html_escape::decode_html_entities(artist)
        .to_lowercase()
        .replace(" feat. ", ",")
        .replace(" ft. ", ",")
        .replace(" & ", ",")
        .replace(" and ", ",");
    let mut names: Vec<_> = clean
        .split(',')
        .map(crate::providers::strict_match::normalized)
        .filter(|name| !name.is_empty())
        .collect();
    names.sort();
    names.dedup();
    names
}
pub fn song_key(title: &str, artist: &str) -> Option<String> {
    let title = crate::providers::strict_match::normalized(title);
    let credits = artist_credits(artist);
    if title.is_empty() || credits.is_empty() {
        None
    } else {
        Some(format!("{title}|{}", credits.join("|")))
    }
}
fn recording_credits(title: &str, artist: &str) -> Vec<String> {
    let title = crate::providers::strict_match::normalized(title);
    if title.is_empty() {
        return vec![];
    }
    artist_credits(artist)
        .into_iter()
        .map(|artist| format!("{title}|{artist}"))
        .collect()
}
#[derive(Clone, Debug, FromRow)]
pub struct SongIdentity {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub source_id: Option<String>,
    pub language: Option<String>,
    pub year: Option<i64>,
}
impl SongIdentity {
    fn key(&self) -> String {
        song_key(&self.title, &self.artist).unwrap_or_else(|| format!("id:{}", self.id))
    }
}
#[derive(Clone, Debug, Default, FromRow)]
pub struct ListeningSignal {
    pub play_count: i64,
    pub completion_count: i64,
    pub skip_count: i64,
    pub last_played_at: Option<i64>,
    pub manual_like: i64,
}
impl ListeningSignal {
    fn add(&mut self, other: &Self) {
        self.play_count = self.play_count.saturating_add(other.play_count.max(0));
        self.completion_count = self
            .completion_count
            .saturating_add(other.completion_count.max(0));
        self.skip_count = self.skip_count.saturating_add(other.skip_count.max(0));
        self.last_played_at = self.last_played_at.max(other.last_played_at);
        self.manual_like = if self.manual_like == -1 || other.manual_like == -1 {
            -1
        } else {
            self.manual_like.max(other.manual_like)
        };
    }
}
#[derive(Clone, Debug, FromRow)]
struct FeedbackPreference {
    song_key: String,
    track_id: String,
    source_id: Option<String>,
    title: String,
    artist: String,
    manual_like: i64,
    updated_at: i64,
}
fn same_recording(a_title: &str, a_artist: &str, b_title: &str, b_artist: &str) -> bool {
    let a = recording_credits(a_title, a_artist);
    let b = recording_credits(b_title, b_artist);
    a.iter().any(|credit| b.contains(credit))
}
#[derive(Default)]
pub struct SongSignals {
    artist_releases: HashSet<String>,
    preferences: Vec<FeedbackPreference>,
    preference_lookup: HashMap<String, usize>,
    pub identities: HashMap<String, SongIdentity>,
    by_song: HashMap<String, ListeningSignal>,
    by_source: HashMap<String, ListeningSignal>,
    blocked_credits: HashSet<String>,
    blocked_songs: HashSet<String>,
    blocked_sources: HashSet<String>,
    blocked_ids: HashSet<String>,
}
impl SongSignals {
    pub async fn load(pool: &SqlitePool, user: &str) -> AppResult<Self> {
        let mut index = Self::default();
        index.artist_releases = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT preferred_source_id FROM catalog_artist_editions",
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .collect();
        index.preferences=sqlx::query_as::<_,FeedbackPreference>("SELECT song_key,track_id,source_id,title,artist,manual_like,updated_at FROM recommendation_feedback WHERE user_id=?").bind(user).fetch_all(pool).await?;
        for (position, preference) in index.preferences.iter().enumerate() {
            let mut keys = vec![format!("id:{}", preference.track_id)];
            if let Some(source) = &preference.source_id {
                keys.push(format!("source:{source}"));
            }
            keys.extend(
                recording_credits(&preference.title, &preference.artist)
                    .into_iter()
                    .map(|key| format!("credit:{key}")),
            );
            for key in keys {
                let replace = index
                    .preference_lookup
                    .get(&key)
                    .is_none_or(|old| index.preferences[*old].updated_at <= preference.updated_at);
                if replace {
                    index.preference_lookup.insert(key, position);
                }
            }
        }
        let external=sqlx::query_as::<_,SongIdentity>(
            "SELECT e.id,e.title,e.artist,COALESCE(s.provider_id,CASE WHEN e.provider='jiosaavn' THEN e.provider_id END) as source_id,
                    COALESCE(e.language,canonical.language,json_extract(s.metadata_json,'$.language')) as language,COALESCE(e.release_year,canonical.release_year,json_extract(s.metadata_json,'$.year')) as year
             FROM external_tracks e LEFT JOIN online_track_sources s ON s.track_id=e.id
             LEFT JOIN external_tracks canonical ON canonical.provider='jiosaavn' AND canonical.provider_id=s.provider_id")
            .fetch_all(pool).await?;
        for song in external {
            index.identities.insert(song.id.clone(), song);
        }
        let tracks=sqlx::query_as::<_,SongIdentity>(
            "SELECT t.id,t.title,COALESCE(a.name,'') as artist,NULL as source_id,NULL as language,t.year
             FROM tracks t LEFT JOIN artists a ON a.id=t.artist_id")
            .fetch_all(pool).await?;
        for song in tracks {
            index.identities.entry(song.id.clone()).or_insert(song);
        }
        let stats: Vec<(String,i64,i64,i64,Option<i64>,i64)>=sqlx::query_as(
            "SELECT track_id,COALESCE(play_count,0),COALESCE(completion_count,0),COALESCE(skip_count,0),last_played_at,COALESCE(manual_like,0)
             FROM track_statistics WHERE user_id=?")
            .bind(user).fetch_all(pool).await?;
        let liked: Vec<String>=sqlx::query_scalar("SELECT DISTINCT pt.track_id FROM playlist_tracks pt JOIN playlists p ON p.id=pt.playlist_id WHERE p.user_id=? AND p.name='Liked Songs' AND p.is_smart_mix=0").bind(user).fetch_all(pool).await?;
        let liked: HashSet<_> = liked.into_iter().collect();
        let mut accounted = HashSet::new();
        for (id, plays, completions, skips, last, feedback) in stats {
            accounted.insert(id.clone());
            let feedback = if feedback == -1 {
                -1
            } else if feedback == 1 || liked.contains(&id) {
                1
            } else {
                0
            };
            index.record(
                &id,
                ListeningSignal {
                    play_count: plays,
                    completion_count: completions,
                    skip_count: skips,
                    last_played_at: last,
                    manual_like: feedback,
                },
            );
        }
        for id in liked.difference(&accounted) {
            index.record(
                id,
                ListeningSignal {
                    manual_like: 1,
                    ..Default::default()
                },
            );
        }
        for preference in &index.preferences {
            if preference.manual_like != -1 {
                continue;
            }
            index.blocked_ids.insert(preference.track_id.clone());
            if let Some(key) = song_key(&preference.title, &preference.artist) {
                index.blocked_songs.insert(key);
            }
            if let Some(source) = &preference.source_id {
                index.blocked_sources.insert(source.clone());
            }
            index
                .blocked_credits
                .extend(recording_credits(&preference.title, &preference.artist));
        }
        Ok(index)
    }
    fn record(&mut self, id: &str, mut signal: ListeningSignal) {
        let Some(identity) = self.identities.get(id) else {
            return;
        };
        if let Some(preference) = self.preference(identity) {
            signal.manual_like = preference.manual_like;
        }
        let key = identity.key();
        if signal.manual_like == -1 {
            self.blocked_songs.insert(key.clone());
            self.blocked_ids.insert(id.into());
            for credit in recording_credits(&identity.title, &identity.artist) {
                self.blocked_credits.insert(credit);
            }
            if let Some(source) = &identity.source_id {
                self.blocked_sources.insert(source.clone());
            }
        }
        self.by_song.entry(key).or_default().add(&signal);
        if let Some(source) = &identity.source_id {
            self.by_source
                .entry(source.clone())
                .or_default()
                .add(&signal);
        }
    }
    pub fn blocked(&self, id: &str, title: &str, artist: &str) -> bool {
        recording_credits(title, artist)
            .iter()
            .any(|key| self.blocked_credits.contains(key))
            || self.blocked_ids.contains(id)
            || song_key(title, artist).is_some_and(|key| self.blocked_songs.contains(&key))
            || self.identities.get(id).is_some_and(|song| {
                self.blocked_songs.contains(&song.key())
                    || song
                        .source_id
                        .as_ref()
                        .is_some_and(|source| self.blocked_sources.contains(source))
            })
    }
    pub fn release_rank(
        &self,
        id: &str,
        title: &str,
        album: Option<&str>,
        year: Option<i64>,
    ) -> (bool, u8, i64) {
        let verified = self
            .identities
            .get(id)
            .and_then(|song| song.source_id.as_ref())
            .is_some_and(|source| self.artist_releases.contains(source));
        let (quality, year) = crate::providers::releases::release_rank(title, album, year);
        (verified, quality, year)
    }

    pub fn blocked_id(&self, id: &str) -> bool {
        self.identities
            .get(id)
            .map(|song| self.blocked(id, &song.title, &song.artist))
            .unwrap_or_else(|| self.blocked_ids.contains(id))
    }
    pub fn signal(&self, id: &str, title: &str, artist: &str) -> ListeningSignal {
        let song = song_key(title, artist).and_then(|key| self.by_song.get(&key));
        let source = self
            .identities
            .get(id)
            .and_then(|song| song.source_id.as_ref())
            .and_then(|source| self.by_source.get(source));
        match (song, source) {
            (Some(a), Some(b)) => {
                if a.play_count >= b.play_count {
                    a.clone()
                } else {
                    b.clone()
                }
            }
            (Some(a), None) | (None, Some(a)) => a.clone(),
            _ => ListeningSignal::default(),
        }
    }
    fn preference(&self, song: &SongIdentity) -> Option<&FeedbackPreference> {
        let mut keys = vec![format!("id:{}", song.id)];
        if let Some(source) = &song.source_id {
            keys.push(format!("source:{source}"));
        }
        keys.extend(
            recording_credits(&song.title, &song.artist)
                .into_iter()
                .map(|key| format!("credit:{key}")),
        );
        keys.iter()
            .filter_map(|key| self.preference_lookup.get(key))
            .map(|index| &self.preferences[*index])
            .max_by_key(|preference| preference.updated_at)
    }
    pub fn related_feedback_keys(&self, id: &str) -> Vec<String> {
        self.identities
            .get(id)
            .map(|song| {
                self.preferences
                    .iter()
                    .filter(|preference| {
                        preference.track_id == song.id
                            || (song.source_id.is_some() && preference.source_id == song.source_id)
                            || same_recording(
                                &song.title,
                                &song.artist,
                                &preference.title,
                                &preference.artist,
                            )
                    })
                    .map(|preference| preference.song_key.clone())
                    .collect()
            })
            .unwrap_or_default()
    }
    pub fn related_ids(&self, id: &str) -> Vec<String> {
        let Some(target) = self.identities.get(id) else {
            return vec![id.into()];
        };
        let key = target.key();
        self.identities
            .values()
            .filter(|song| {
                song.id == id
                    || song.key() == key
                    || recording_credits(&song.title, &song.artist)
                        .iter()
                        .any(|credit| {
                            recording_credits(&target.title, &target.artist).contains(credit)
                        })
                    || (target.source_id.is_some() && song.source_id == target.source_id)
            })
            .map(|song| song.id.clone())
            .collect()
    }
    /// Only recommendation responses call this. Manual search/library remain intact.
    pub fn filter(&self, value: serde_json::Value) -> Option<serde_json::Value> {
        match value {
            serde_json::Value::Array(items) => Some(serde_json::Value::Array(
                items
                    .into_iter()
                    .filter_map(|item| self.filter(item))
                    .collect(),
            )),
            serde_json::Value::Object(mut object) => {
                let id = object
                    .get("track_id")
                    .or_else(|| object.get("external_track_id"))
                    .or_else(|| object.get("id"))
                    .and_then(|value| value.as_str());
                if object.contains_key("title") && id.is_some_and(|id| self.blocked_id(id)) {
                    return None;
                }
                for value in object.values_mut() {
                    *value = self.filter(value.take()).unwrap_or(serde_json::Value::Null);
                }
                Some(serde_json::Value::Object(object))
            }
            value => Some(value),
        }
    }
}

/// Preserve an imported dislike once, without allowing neutral listening snapshots
/// to erase a deliberate choice. Explicit local feedback remains authoritative.
pub async fn remember_imported_dislikes(pool: &SqlitePool, user: &str) -> AppResult<()> {
    let index = SongSignals::load(pool, user).await?;
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT track_id FROM track_statistics WHERE user_id=? AND manual_like=-1",
    )
    .bind(user)
    .fetch_all(pool)
    .await?;
    for id in ids {
        let Some(song) = index.identities.get(&id) else {
            continue;
        };
        if index.preference(song).is_some() {
            continue;
        }
        sqlx::query("INSERT INTO recommendation_feedback(user_id,song_key,track_id,source_id,title,artist,manual_like,updated_at) VALUES(?,?,?,?,?,?,-1,?) ON CONFLICT(user_id,song_key) DO NOTHING")
            .bind(user).bind(song.key()).bind(&song.id).bind(&song.source_id).bind(&song.title).bind(&song.artist).bind(chrono::Utc::now().timestamp_millis()).execute(pool).await?;
    }
    let stats: Vec<String> =
        sqlx::query_scalar("SELECT track_id FROM track_statistics WHERE user_id=?")
            .bind(user)
            .fetch_all(pool)
            .await?;
    for id in stats {
        if let Some(preference) = index
            .identities
            .get(&id)
            .and_then(|song| index.preference(song))
        {
            sqlx::query("UPDATE track_statistics SET manual_like=? WHERE user_id=? AND track_id=?")
                .bind(preference.manual_like)
                .bind(user)
                .bind(id)
                .execute(pool)
                .await?;
        }
    }
    Ok(())
}

pub use crate::providers::releases::release_preference;

#[cfg(test)]
mod release_tests {
    use super::*;
    #[test]
    fn original_single_outranks_compilation_without_swapping_artwork() {
        assert!(
            release_preference("Harleys In Hawaii", Some("Harleys In Hawaii"))
                > release_preference("Harleys In Hawaii", Some("POP 2010"))
        );
        assert!(
            release_preference("Breathe Me", Some("Colour The Small One"))
                > release_preference("Breathe Me", Some("Summer Hits"))
        );
    }
}
