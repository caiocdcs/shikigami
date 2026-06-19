-- Rename check_ins.comments -> message.
-- The column already existed (initial_db) but was never populated and was named
-- for a generic "comments" concept. The failure-context feature treats it as the
-- check-in message (the reason a job reported failure, or context on a ping).
ALTER TABLE check_ins RENAME COLUMN comments TO message;
