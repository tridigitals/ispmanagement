-- Align email_outbox ID types with the rest of the schema (text IDs)
-- Existing installations used UUID columns; convert to text to match users/tenants tables.

ALTER TABLE IF EXISTS email_outbox
  ALTER COLUMN id TYPE text USING id::text;

ALTER TABLE IF EXISTS email_outbox
  ALTER COLUMN tenant_id TYPE text USING tenant_id::text;

