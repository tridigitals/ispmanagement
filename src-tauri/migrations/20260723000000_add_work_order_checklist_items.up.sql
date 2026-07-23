-- +migrate Up
CREATE TABLE IF NOT EXISTS work_order_checklist_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    work_order_id UUID NOT NULL REFERENCES installation_work_orders(id) ON DELETE CASCADE,
    label TEXT NOT NULL,
    sort_order INT NOT NULL DEFAULT 0,
    is_completed BOOLEAN NOT NULL DEFAULT false,
    completed_at TIMESTAMPTZ,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_wo_checklist_work_order ON work_order_checklist_items(work_order_id);
CREATE INDEX IF NOT EXISTS idx_wo_checklist_tenant ON work_order_checklist_items(tenant_id);

-- +migrate Down
DROP TABLE IF EXISTS work_order_checklist_items;
