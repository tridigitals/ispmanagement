-- Email outbox: optional HTML body for richer templates (e.g., announcements)

ALTER TABLE email_outbox
  ADD COLUMN IF NOT EXISTS body_html text NULL;

