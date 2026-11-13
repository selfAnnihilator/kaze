-- Auto-tuned exploration (ADR 0030).
-- Flow records whether each enqueued pick was an exploration ("explore") or a
-- familiar ("familiar") slot; the history service stamps it onto the finished
-- listen so the explore/familiar success rates can be compared.
ALTER TABLE playback_history ADD COLUMN rec_slot TEXT;
CREATE TABLE IF NOT EXISTS flow_slot_assignments (
    user_id TEXT NOT NULL,
    track_id TEXT NOT NULL,
    slot TEXT NOT NULL,
    assigned_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, track_id)
);
CREATE INDEX IF NOT EXISTS idx_history_rec_slot ON playback_history(rec_slot, started_at);
