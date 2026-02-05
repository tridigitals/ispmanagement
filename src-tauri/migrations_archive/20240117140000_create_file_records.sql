-- Add file_records table

CREATE TABLE IF NOT EXISTS file_records (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    name TEXT NOT NULL,
    original_name TEXT NOT NULL,
    path TEXT NOT NULL,
    size INTEGER NOT NULL,
    content_type TEXT NOT NULL,
    uploaded_by TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(tenant_id) REFERENCES tenants(id),
    FOREIGN KEY(uploaded_by) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_file_records_tenant_id ON file_records(tenant_id);
