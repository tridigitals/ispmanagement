-- Revert email_outbox columns back to UUID (only safe if values are valid UUID strings)

ALTER TABLE IF EXISTS email_outbox
  ALTER COLUMN id TYPE uuid USING id::uuid;

ALTER TABLE IF EXISTS email_outbox
  ALTER COLUMN tenant_id TYPE uuid USING tenant_id::uuid;

