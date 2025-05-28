ALTER TABLE players ADD COLUMN total_messages INTEGER NOT NULL DEFAULT 0;
ALTER TABLE infection_records ADD COLUMN target_messages INTEGER NOT NULL;
