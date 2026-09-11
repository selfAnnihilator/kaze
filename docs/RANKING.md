# Ranking & Listening Statistics Specification

## 1. Meaningful Play Definition

Accidental clicks and rapid track skips should not pollute listening history or inflate popularity metrics. The system evaluates whether a listening session represents an intentional, engaged play using configurable thresholds in `HistoryConfig`:

$$\text{is\_meaningful} = (\text{seconds\_listened} \ge 30.0) \lor \left(\frac{\text{seconds\_listened}}{\text{track\_duration}} \ge 0.50\right)$$

* If a user listens for at least **30 seconds**, it counts as a meaningful play.
* For short songs (<60s), listening to at least **50% of the duration** qualifies as a meaningful play.
* If playback stops or skips before meeting either threshold, the session is marked as `skipped = 1` and does not increment `play_count`.

---

## 2. Multi-Factor Ranking Formula

Rankings do not rely solely on raw play counts. The ranking score incorporates multi-signal engagement, listening depth, user sentiment, and recency decay:

$$\text{Score} = (w_{\text{play}} \times P) + (w_{\text{duration}} \times \ln(1 + D)) + (w_{\text{comp}} \times C) + (w_{\text{recency}} \times R) + (w_{\text{pref}} \times L) - (w_{\text{skip}} \times S)$$

### Factor Breakdown:
1. **$P$ (Play Count)**: Total meaningful plays within the target time window.
2. **$D$ (Listening Duration in Minutes)**: Log-damped listening duration ($\ln(1 + \text{minutes})$) to prevent 20-minute ambient tracks from drowning out shorter songs.
3. **$C$ (Completion Rate)**: Ratio of completed plays ($\text{completion\_count} / \max(1, \text{play\_count})$).
4. **$R$ (Recency Factor)**: Exponential decay based on days elapsed since the track was last played:
   $$R = e^{-\lambda \times \Delta t_{\text{days}}}$$
   where $\lambda = 0.05$ (half-life of ~14 days).
5. **$L$ (User Preference)**: Explicit sentiment:
   * $+1.0$ if Liked
   * $-1.0$ if Disliked
   * $0.0$ if Neutral
6. **$S$ (Skip Rate)**: Frequency of skips before the meaningful play threshold ($\text{skip\_count} / \max(1, \text{play\_count} + \text{skip\_count})$).

### Default Configurable Weights:
* `play_count_weight`: $1.0$
* `listening_duration_weight`: $0.8$
* `completion_weight`: $1.2$
* `recency_weight`: $1.5$
* `user_preference_weight`: $2.0$
* `skip_penalty`: $1.0$

---

## 3. Time Windows

Rolling ranking computations support 6 standard temporal horizons:
* **Today**: From 00:00:00 UTC of the current calendar day.
* **Last 7 Days**: Previous 168 hours.
* **Last 30 Days**: Previous 720 hours.
* **Last 6 Months**: Previous 180 days.
* **Last 1 Year**: Previous 365 days.
* **All Time**: Entire recorded playback history.
