-- WhatsApp Gateway delivery audit log

CREATE TABLE IF NOT EXISTS whatsapp_delivery_logs (
  id UUID PRIMARY KEY,
  tenant_id UUID NULL,
  scope TEXT NOT NULL,
  event_code TEXT NOT NULL,
  provider TEXT NOT NULL,
  recipient_user_id UUID NULL,
  recipient_phone TEXT NOT NULL,
  status TEXT NOT NULL,
  error_summary TEXT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_whatsapp_delivery_logs_tenant_created
  ON whatsapp_delivery_logs (tenant_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_whatsapp_delivery_logs_event
  ON whatsapp_delivery_logs (scope, event_code);

CREATE INDEX IF NOT EXISTS idx_whatsapp_delivery_logs_recipient_user
  ON whatsapp_delivery_logs (recipient_user_id);
