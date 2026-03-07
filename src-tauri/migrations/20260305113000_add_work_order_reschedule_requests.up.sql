CREATE TABLE IF NOT EXISTS work_order_reschedule_requests (
  id TEXT PRIMARY KEY,
  tenant_id TEXT NOT NULL,
  work_order_id TEXT NOT NULL,
  subscription_id TEXT NOT NULL,
  requested_by TEXT NOT NULL,
  requested_schedule_at TEXT NOT NULL,
  reason TEXT,
  status TEXT NOT NULL DEFAULT 'pending',
  reviewed_by TEXT,
  reviewed_at TEXT,
  review_notes TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT chk_work_order_reschedule_requests_status
    CHECK (status IN ('pending', 'approved', 'rejected', 'cancelled')),
  CONSTRAINT fk_work_order_reschedule_requests_work_order
    FOREIGN KEY (work_order_id) REFERENCES installation_work_orders(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_worr_tenant_work_order
  ON work_order_reschedule_requests(tenant_id, work_order_id);

CREATE INDEX IF NOT EXISTS idx_worr_tenant_status
  ON work_order_reschedule_requests(tenant_id, status);

CREATE UNIQUE INDEX IF NOT EXISTS uq_worr_one_pending_per_work_order
  ON work_order_reschedule_requests(tenant_id, work_order_id)
  WHERE status = 'pending';
