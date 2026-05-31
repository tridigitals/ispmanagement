-- Drop attachments table; cascade FK is on the outbox_id reference, so this
-- is safe regardless of whether email_outbox itself is being dropped.

DROP INDEX IF EXISTS idx_email_outbox_attachments_outbox_id;
DROP TABLE IF EXISTS email_outbox_attachments;
