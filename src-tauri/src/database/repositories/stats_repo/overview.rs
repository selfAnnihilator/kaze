//! The Stats overview: period totals, graphs and rankings.
use super::*;

impl SqliteStatsRepository {
    pub(super) async fn get_stats_overview_query(
        &self,
        user_id: &str,
        user_joined_at: i64,
        app_start_date: i64,
        selected_year: Option<i32>,
        selected_month: Option<u32>,
        weights: &RankingWeightsConfig,
    ) -> AppResult<StatsOverview> {
        let now = chrono::Local::now();
        let current_year = now.date_naive().year();
        let target_year = selected_year.unwrap_or(current_year);
        let target_month = selected_month.unwrap_or_else(|| {
            if target_year == current_year {
                now.month()
            } else {
                12
            }
        });

        // Query lifetime listening seconds from track_statistics (all-time aggregate, includes cloud-restored data)
        let lifetime_seconds: f64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(total_time_listened), 0.0) FROM track_statistics WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0.0);

        // Daily aggregates are the only timeline authority, including restored installs.
        let min_date: Option<String> = sqlx::query_scalar(
            "SELECT MIN(stat_date) FROM daily_user_stats WHERE user_id = ?")
            .bind(user_id).fetch_one(&self.pool).await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let history_started_at = min_date.and_then(|date| {
            use chrono::TimeZone;
            chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok()
                .and_then(|day| day.and_hms_opt(0, 0, 0))
                .and_then(|midnight| chrono::Local.from_local_datetime(&midnight).earliest())
                .map(|dt| dt.timestamp())
        });
        let year_rows: Vec<(Option<i32>,)> = sqlx::query_as(
            "SELECT DISTINCT CAST(substr(stat_date, 1, 4) AS INTEGER) as yr
             FROM daily_user_stats WHERE user_id = ? ORDER BY yr DESC")
            .bind(user_id).fetch_all(&self.pool).await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut available_years: Vec<i32> = year_rows.into_iter().filter_map(|r| r.0).collect();
        if !available_years.contains(&current_year) {
            available_years.push(current_year);
        }
        available_years.sort_by(|a, b| b.cmp(a));

        let available_months: Vec<u32> = (1..=12).collect();

        // Calculate monthly graph and seconds for the selected month
        let days_in_month = match target_month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                let is_leap = (target_year % 4 == 0 && target_year % 100 != 0) || (target_year % 400 == 0);
                if is_leap { 29 } else { 28 }
            }
            _ => 30,
        };

        let start_date_str = format!("{:04}-{:02}-01", target_year, target_month);
        let end_date_str = format!("{:04}-{:02}-{:02}", target_year, target_month, days_in_month);

        let daily_rows: Vec<(String, f64)> = sqlx::query_as(
            "SELECT
                stat_date as day_date,
                COALESCE(SUM(listening_seconds), 0.0) as total_seconds
             FROM daily_user_stats
             WHERE user_id = ? AND stat_date >= ? AND stat_date <= ?
             GROUP BY stat_date"
        )
        .bind(user_id)
        .bind(&start_date_str)
        .bind(&end_date_str)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        let mut day_map = std::collections::HashMap::new();
        for (d, s) in daily_rows {
            day_map.insert(d, s);
        }

        let mut monthly_graph = Vec::with_capacity(days_in_month as usize);
        let mut monthly_seconds = 0.0;
        for day in 1..=days_in_month {
            let date_str = format!("{:04}-{:02}-{:02}", target_year, target_month, day);
            let seconds = day_map.get(&date_str).copied().unwrap_or(0.0);
            monthly_seconds += seconds;
            monthly_graph.push(DailyListeningPoint {
                day,
                date: date_str,
                total_seconds: seconds,
            });
        }

        // 3. Current year stats from daily_user_stats
        let today_str = chrono::Local::now().format("%Y-%m-%d").to_string();
        let week_ago_str = (chrono::Local::now() - ChronoDuration::days(6)).format("%Y-%m-%d").to_string();
        let year_start_str = format!("{:04}-01-01", target_year);
        let year_end_str = format!("{:04}-12-31", target_year);

        let daily_seconds: f64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(listening_seconds), 0.0) FROM daily_user_stats
             WHERE user_id = ? AND stat_date = ?"
        )
        .bind(user_id)
        .bind(&today_str)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0.0);

        let weekly_seconds: f64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(listening_seconds), 0.0) FROM daily_user_stats
             WHERE user_id = ? AND stat_date >= ? AND stat_date <= ?"
        )
        .bind(user_id)
        .bind(&week_ago_str)
        .bind(&today_str)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0.0);

        let total_year_seconds: f64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(listening_seconds), 0.0) FROM daily_user_stats
             WHERE user_id = ? AND stat_date >= ? AND stat_date <= ?"
        )
        .bind(user_id)
        .bind(&year_start_str)
        .bind(&year_end_str)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0.0);

        // For all-time / current year rankings, base on track_statistics to encompass cloud-restored and local playback
        // Top 10 songs; top 5 artists + top 5 genres share the same card height.
        let top_songs = self.get_all_time_ranked_tracks(user_id, weights, 10).await.unwrap_or_else(|e| {
            tracing::warn!(%e, "stats overview: top songs query failed");
            Vec::new()
        });
        let top_artists = self.get_all_time_ranked_artists(user_id, weights, 5).await.unwrap_or_else(|e| {
            tracing::warn!(%e, "stats overview: top artists query failed");
            Vec::new()
        });
        let top_genres = self.get_all_time_ranked_genres(user_id, weights, 5).await.unwrap_or_else(|e| {
            tracing::warn!(%e, "stats overview: top genres query failed");
            Vec::new()
        });
        let top_days = self.get_top_listening_days(user_id, 5).await.unwrap_or_else(|e| {
            tracing::warn!(%e, "stats overview: top days query failed");
            Vec::new()
        });

        Ok(StatsOverview {
            app_start_date,
            user_joined_date: user_joined_at,
            current_year,
            selected_year: target_year,
            available_years,
            selected_month: target_month,
            available_months,
            daily_seconds,
            weekly_seconds,
            monthly_seconds,
            total_year_seconds,
            lifetime_seconds,
            history_started_at,
            monthly_graph,
            top_songs,
            top_artists,
            top_genres,
            top_days,
        })
    }
}
