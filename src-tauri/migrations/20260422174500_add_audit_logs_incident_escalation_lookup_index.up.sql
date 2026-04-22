CREATE INDEX IF NOT EXISTS idx_audit_logs_incident_escalation_lookup
    ON public.audit_logs (tenant_id, resource, action, resource_id, created_at DESC);
