-- Announcements / Broadcasts
-- Scope:
-- - tenant_id NULL => global announcement
-- - tenant_id set  => tenant-scoped announcement

CREATE TABLE IF NOT EXISTS announcements (
  id text PRIMARY KEY,
  tenant_id text NULL,
  created_by text NULL,
  title text NOT NULL,
  body text NOT NULL,
  severity text NOT NULL DEFAULT 'info', -- info|success|warning|error
  audience text NOT NULL DEFAULT 'all',  -- all|admins
  mode text NOT NULL DEFAULT 'post', -- post|banner
  format text NOT NULL DEFAULT 'plain', -- plain|markdown
  deliver_in_app boolean NOT NULL DEFAULT true,
  deliver_email boolean NOT NULL DEFAULT false,
  starts_at timestamp with time zone NOT NULL DEFAULT now(),
  ends_at timestamp with time zone NULL,
  notified_at timestamp with time zone NULL,
  created_at timestamp with time zone NOT NULL,
  updated_at timestamp with time zone NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_announcements_scope_time
  ON announcements (tenant_id, starts_at, ends_at, notified_at);

CREATE TABLE IF NOT EXISTS announcement_dismissals (
  id text PRIMARY KEY,
  announcement_id text NOT NULL,
  user_id text NOT NULL,
  dismissed_at timestamp with time zone NOT NULL,
  UNIQUE(user_id, announcement_id)
);

CREATE INDEX IF NOT EXISTS idx_announcement_dismissals_user
  ON announcement_dismissals (user_id, announcement_id);
