-- Speaker identification (diarization) feature toggle.
-- Single-row table keyed by id = '1'; default disabled (experimental, opt-in).
CREATE TABLE IF NOT EXISTS diarization_settings (
    id TEXT PRIMARY KEY,
    enabled INTEGER NOT NULL DEFAULT 0
);

INSERT OR IGNORE INTO diarization_settings (id, enabled) VALUES ('1', 0);
