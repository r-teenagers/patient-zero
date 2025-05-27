CREATE TABLE players (
		id TEXT PRIMARY KEY,
		infected BOOL NOT NULL
);

CREATE TABLE infection_records (
		id INTEGER PRIMARY KEY,
		event TEXT NOT NULL CHECK(event IN ("infected", "cured")),
		target TEXT NOT NULL,
		source TEXT,
		reason TEXT,
		recorded_at INTEGER NOT NULL,
		FOREIGN KEY (target) REFERENCES players (id),
		FOREIGN KEY (source) REFERENCES players (id)
);
