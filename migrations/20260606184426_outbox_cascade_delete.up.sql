-- Rebuild notification_outbox with ON DELETE CASCADE on monitor_id
-- SQLite does not support ALTER CONSTRAINT, so we recreate the table.

ALTER TABLE notification_outbox RENAME TO notification_outbox_old;

CREATE TABLE notification_outbox (
    id TEXT PRIMARY KEY,
    monitor_id TEXT NOT NULL,
    integration_id TEXT NOT NULL,
    message TEXT NOT NULL,
    retry_count INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY(monitor_id) REFERENCES monitors(id) ON DELETE CASCADE,
    FOREIGN KEY(integration_id) REFERENCES integrations(id) ON DELETE CASCADE
);

INSERT INTO notification_outbox SELECT * FROM notification_outbox_old;

DROP TABLE notification_outbox_old;
