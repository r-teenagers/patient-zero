CREATE TABLE players (
	id TEXT PRIMARY KEY NOT NULL,
	infected BOOL NOT NULL DEFAULT FALSE,
	-- all messages, saved for statistics reasons
	total_messages INTEGER NOT NULL DEFAULT 0,
	-- sanitized messages, saved for auto-cure
	sanitized_messages INTEGER NOT NULL DEFAULT 0,
	-- timestamp of the last sanitized message being saved
	last_action INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE infection_records (
	id INTEGER PRIMARY KEY NOT NULL,
	event TEXT NOT NULL CHECK(event IN ("infected", "cured")),
	target TEXT NOT NULL,
	source TEXT,
	reason TEXT,
	recorded_at INTEGER NOT NULL DEFAULT (unixepoch()),
	target_total_messages INTEGER NOT NULL,
	target_sanitized_messages INTEGER NOT NULL,
	FOREIGN KEY (target) REFERENCES players (id),
	FOREIGN KEY (source) REFERENCES players (id)
);

CREATE INDEX idx_ir_target ON infection_records (target);
CREATE INDEX idx_ir_source ON infection_records (source);
