-- Revert: drop the per-monitor timezone column.
ALTER TABLE monitors DROP COLUMN timezone;
