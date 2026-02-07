-- Email Outbox (queued email delivery with retries)
-- Postgres only (SQLite may ignore in builds without postgres feature)

CREATE TABLE IF NOT EXISTS email_outbox (
  id UUID PRIMARY KEY,
  tenant_id UUID NULL,
  to_email TEXT NOT NULL,
  subject TEXT NOT NULL,
  body TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'queued', -- queued | sending | sent | failed
  attempts INTEGER NOT NULL DEFAULT 0,
  max_attempts INTEGER NOT NULL DEFAULT 5,
  scheduled_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  last_error TEXT NULL,
  sent_at TIMESTAMPTZ NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_email_outbox_status_scheduled
  ON email_outbox (status, scheduled_at);

CREATE INDEX IF NOT EXISTS idx_email_outbox_tenant
  ON email_outbox (tenant_id);

