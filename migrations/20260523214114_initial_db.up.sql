-- Add up migration script here
CREATE TABLE monitors (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    slug TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL DEFAULT 'active',
    schedule_type TEXT NOT NULL,
    cron_expr TEXT,
    interval_seconds INTEGER,
    grace_seconds INTEGER NOT NULL,
    last_pinged_at TEXT,
    next_expected_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE check_ins (
    id TEXT PRIMARY KEY,
    monitor_id TEXT NOT NULL,
    checked_in_at TEXT NOT NULL DEFAULT (datetime('now')),
    outcome TEXT NOT NULL,
    comments TEXT,
    FOREIGN KEY(monitor_id) REFERENCES monitors(id) ON DELETE CASCADE
);

CREATE TABLE integrations (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    channel_type TEXT NOT NULL,
    config_json TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE monitor_integrations (
    monitor_id TEXT NOT NULL,
    integration_id TEXT NOT NULL,
    PRIMARY KEY (monitor_id, integration_id),
    FOREIGN KEY(monitor_id) REFERENCES monitors(id) ON DELETE CASCADE,
    FOREIGN KEY(integration_id) REFERENCES integrations(id) ON DELETE CASCADE
);

CREATE TABLE notification_outbox (
    id TEXT PRIMARY KEY,
    monitor_id TEXT NOT NULL,
    integration_id TEXT NOT NULL,
    message TEXT NOT NULL,
    retry_count INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY(monitor_id) REFERENCES monitors(id),
    FOREIGN KEY(integration_id) REFERENCES integrations(id)
);
