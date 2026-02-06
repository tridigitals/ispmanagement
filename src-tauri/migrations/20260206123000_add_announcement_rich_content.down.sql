DROP INDEX IF EXISTS idx_announcements_cover;

ALTER TABLE announcements
  DROP COLUMN IF EXISTS cover_file_id;

