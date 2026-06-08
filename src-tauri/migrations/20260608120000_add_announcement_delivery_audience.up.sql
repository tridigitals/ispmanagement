-- Enhanced announcement delivery channels and audience targeting
-- Adds WhatsApp + push delivery options and package-level audience filtering.

ALTER TABLE announcements
  ADD COLUMN IF NOT EXISTS deliver_whatsapp boolean NOT NULL DEFAULT false;

ALTER TABLE announcements
  ADD COLUMN IF NOT EXISTS deliver_push boolean NOT NULL DEFAULT false;

-- Optional: restrict audience to subscribers of a specific package
ALTER TABLE announcements
  ADD COLUMN IF NOT EXISTS target_package_id text NULL;

CREATE INDEX IF NOT EXISTS idx_announcements_target_package
  ON announcements (target_package_id);
