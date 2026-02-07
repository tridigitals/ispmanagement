-- Announcement email delivery options
-- deliver_email_force = true => ignore user email notification preferences (force)

ALTER TABLE announcements
  ADD COLUMN IF NOT EXISTS deliver_email_force boolean NOT NULL DEFAULT true;

