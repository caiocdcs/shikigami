# Integrations

Shikigami supports three notification channels: ntfy, gotify, and email.
Each integration is created via `POST /integrations` with a JSON body
containing `name`, `channel`, and `config`.

## ntfy

Push notifications via a [ntfy](https://ntfy.sh) topic.

```json
{
  "name": "alerts",
  "channel": "ntfy",
  "config": {
    "url": "https://ntfy.sh",
    "topic": "homelab",
    "priority": 5,
    "message": "alert"
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `url` | string | ntfy server URL |
| `topic` | string | Topic to publish to |
| `priority` | integer | 1-5 (min-max) |
| `message` | string | Prefix/message template |

## gotify

Push notifications via a [gotify](https://gotify.net) server.

```json
{
  "name": "alerts",
  "channel": "gotify",
  "config": {
    "url": "http://gotify:8080",
    "token": "<app-token>",
    "priority": 5
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `url` | string | Gotify server URL |
| `token` | string | Application token |
| `priority` | integer | 1-7 (min-max) |

## Email

SMTP email alerts. Supports TLS (port 465), STARTTLS (port 587), and
no encryption.

```json
{
  "name": "alerts",
  "channel": "email",
  "config": {
    "smtp_host": "smtp.example.com",
    "smtp_port": 587,
    "smtp_username": "user",
    "smtp_password": "pass",
    "smtp_encryption": "starttls",
    "to": "me@example.com",
    "from": "shikigami@example.com"
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `smtp_host` | string | SMTP server hostname |
| `smtp_port` | integer | Port (465 TLS, 587 STARTTLS, 25 none) |
| `smtp_username` | string | SMTP auth username |
| `smtp_password` | string | SMTP auth password |
| `smtp_encryption` | string | `"tls"`, `"starttls"`, or `"none"` |
| `to` | string | Recipient email address |
| `from` | string | Sender email address |
