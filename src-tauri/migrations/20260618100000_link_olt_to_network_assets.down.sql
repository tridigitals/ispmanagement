-- Sprint B.1 + B.2 rollback

DROP INDEX IF EXISTS idx_network_assets_olt_pon;
ALTER TABLE network_assets DROP COLUMN IF EXISTS pon_port;
ALTER TABLE network_assets DROP COLUMN IF EXISTS olt_id;

DROP INDEX IF EXISTS idx_olt_onu_history_customer;
ALTER TABLE olt_onu_history DROP COLUMN IF EXISTS customer_id;
