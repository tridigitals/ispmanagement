-- Create table for OLT public traffic tokens
-- These allow unauthenticated access to MRTG-style traffic graphs

CREATE TABLE IF NOT EXISTS public.olt_public_tokens (
    id VARCHAR(36) PRIMARY KEY,
    olt_id VARCHAR(36) NOT NULL,
    tenant_id VARCHAR(36) NOT NULL,
    token VARCHAR(64) NOT NULL UNIQUE,
    description TEXT,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ
);

-- Index for fast token lookup (used by public endpoint)
CREATE INDEX IF NOT EXISTS idx_olt_public_tokens_token
    ON public.olt_public_tokens(token)
    WHERE enabled = true;

-- Index for listing tokens per OLT
CREATE INDEX IF NOT EXISTS idx_olt_public_tokens_olt_id
    ON public.olt_public_tokens(olt_id);
