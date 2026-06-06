-- Revert: rebuild notification_outbox without ON DELETE CASCADE on monitor_id

ALTER TABLE notification_outbox RENAME TO notification_outbox_old;

CREATE TABLE notification_outbox (
    id TEXT PRIMARY KEY,
    monitor_id TEXT NOT NULL,
    integration_id TEXT NOT NULL,
    message TEXT NOT NULL,
    retry_count INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY(monitor_id) REFERENCES monitors(id),
    FOREIGN KEY(integration_id) REFERENCES integrations(id) ON DELETE CASCADE
);

INSERT INTO notification_outbox SELECT * FROM notification_outbox_old;

DROP TABLE notification_outbox_old;
