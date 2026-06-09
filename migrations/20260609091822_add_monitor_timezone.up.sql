-- Add per-monitor timezone for cron schedule evaluation.
-- NULL means UTC, preserving existing behaviour for rows created before this migration.
ALTER TABLE monitors ADD COLUMN timezone TEXT;
