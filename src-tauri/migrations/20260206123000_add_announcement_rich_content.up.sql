-- Add rich content support to announcements:
-- - cover_file_id: optional image thumbnail/cover stored in file_records
-- - allow format 'html' (handled at app level)

ALTER TABLE announcements
  ADD COLUMN IF NOT EXISTS cover_file_id text NULL;

CREATE INDEX IF NOT EXISTS idx_announcements_cover
  ON announcements (cover_file_id);

