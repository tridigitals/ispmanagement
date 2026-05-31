-- Email outbox attachments
--
-- Phase 2 of bulk-send-invoice: lets queued emails carry binary attachments
-- (e.g. invoice PDFs). Stored inline as BYTEA so retries are idempotent and
-- there are no orphaned files on disk.
--
-- ID columns are TEXT to match the rest of the email_outbox schema after the
-- 20260207110000 type-realignment migration.

CREATE TABLE IF NOT EXISTS email_outbox_attachments (
  id            TEXT PRIMARY KEY,
  outbox_id     TEXT NOT NULL REFERENCES email_outbox(id) ON DELETE CASCADE,
  filename      TEXT NOT NULL,
  content_type  TEXT NOT NULL,
  content_bytes BYTEA NOT NULL,
  size_bytes    INTEGER NOT NULL,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_email_outbox_attachments_outbox_id
  ON email_outbox_attachments (outbox_id);
