//! Listening evidence for recommendations. Position/EOF alone is never evidence.
pub const MIN_POSITIVE_SECONDS: f64 = 10.0;
pub const BASELINE_SECONDS: f64 = 30.0;
pub const BASELINE_FRACTION: f64 = 0.25;
pub const STRONG_FRACTION: f64 = 0.75;
pub const NEAR_COMPLETE_FRACTION: f64 = 0.95;

/// SQL preselection uses the same constants before applying a result limit.
/// Column expressions are internal aliases, never user input.
pub(crate) fn qualified_sql(seconds: &str, duration: &str) -> String {
    format!("({seconds} >= CASE WHEN {duration}>0 THEN MIN({BASELINE_SECONDS},MAX({MIN_POSITIVE_SECONDS},{BASELINE_FRACTION}*{duration})) ELSE {BASELINE_SECONDS} END)")
}

#[derive(Clone, Copy, Debug)]
pub struct Engagement {
    pub seconds: f64,
    pub fraction: f64,
    pub qualified: bool,
    pub near_complete: bool,
    pub early_skip: bool,
    pub weight: f64,
}
impl Engagement {
    pub fn from_listening(seconds: f64, duration: f64, skipped: bool) -> Self {
        let duration = if duration.is_finite() && duration > 0.0 {
            duration
        } else {
            0.0
        };
        let seconds = if seconds.is_finite() {
            seconds.max(0.0)
        } else {
            0.0
        };
        let seconds = if duration > 0.0 {
            seconds.min(duration)
        } else {
            seconds
        };
        let fraction = if duration > 0.0 {
            seconds / duration
        } else {
            0.0
        };
        // Short songs can qualify sooner, but a few seconds never earn preference.
        let threshold = if duration > 0.0 {
            (duration * BASELINE_FRACTION).clamp(MIN_POSITIVE_SECONDS, BASELINE_SECONDS)
        } else {
            BASELINE_SECONDS
        };
        let qualified = seconds >= threshold;
        let early_skip = skipped && seconds < MIN_POSITIVE_SECONDS;
        let near_complete = qualified && fraction >= NEAR_COMPLETE_FRACTION;
        let weight = if early_skip {
            -0.75
        } else if !qualified {
            0.0
        } else if near_complete {
            1.0
        } else if fraction >= STRONG_FRACTION {
            0.75
        } else {
            0.35
        };
        Self {
            seconds,
            fraction,
            qualified,
            near_complete,
            early_skip,
            weight,
        }
    }
}

/// Read-only scoring projection. Raw listening time remains available for usage charts.
pub(crate) fn scored_history_sql() -> String {
    let duration = "COALESCE(t.duration_secs,e.duration_secs,0)";
    let qualified = qualified_sql("h.seconds_listened", duration);
    format!("SELECT h.*,
        CASE WHEN {qualified} THEN 1 ELSE 0 END AS qualified,
        CASE WHEN {qualified} THEN CASE WHEN {duration}>0 THEN MIN(h.seconds_listened,{duration}) ELSE h.seconds_listened END ELSE 0 END AS qualified_seconds,
        CASE WHEN h.completed=1 AND {qualified} AND {duration}>0 AND h.seconds_listened>={NEAR_COMPLETE_FRACTION}*{duration} THEN 1 ELSE 0 END AS scored_completion,
        CASE WHEN h.skipped=1 AND h.seconds_listened<{MIN_POSITIVE_SECONDS} THEN 1 ELSE 0 END AS scored_skip
        FROM playback_history h LEFT JOIN tracks t ON t.id=h.track_id LEFT JOIN external_tracks e ON e.id=h.track_id")
}

/// Apply local time evidence without rewriting synced cumulative/device totals.
/// Retain only a conservative aggregate remainder for history not stored locally.
pub(crate) fn scored_stats_sql() -> String {
    let history = scored_history_sql();
    format!("SELECT s.user_id,s.track_id,s.manual_like,s.last_played_at,s.total_time_listened,s.play_count,s.completion_count,s.skip_count,
        MAX(0,s.play_count-COALESCE(h.sessions,0))+COALESCE(h.plays,0) AS scored_play_count,
        MAX(0,s.completion_count-COALESCE(h.old_completions,0))+COALESCE(h.completions,0) AS scored_completion_count,
        MAX(0,s.skip_count-COALESCE(h.old_skips,0))+COALESCE(h.skips,0) AS scored_skip_count,
        CASE WHEN s.play_count>COALESCE(h.sessions,0) OR s.completion_count>COALESCE(h.old_completions,0)
            THEN MAX(0,s.total_time_listened-COALESCE(h.raw_seconds,0)) ELSE 0 END + COALESCE(h.qualified_seconds,0) AS qualified_time_listened
        FROM track_statistics s LEFT JOIN (
            SELECT user_id,track_id,COUNT(*) AS sessions,SUM(seconds_listened) AS raw_seconds,
                SUM(qualified_seconds) AS qualified_seconds,SUM(qualified) AS plays,
                SUM(completed) AS old_completions,SUM(skipped) AS old_skips,
                SUM(scored_completion) AS completions,SUM(scored_skip) AS skips
            FROM ({history}) GROUP BY user_id,track_id
        ) h ON h.user_id=s.user_id AND h.track_id=s.track_id")
}
