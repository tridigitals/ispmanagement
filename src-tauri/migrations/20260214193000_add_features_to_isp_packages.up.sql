ALTER TABLE isp_packages
  ADD COLUMN IF NOT EXISTS features text[] NOT NULL DEFAULT '{}';

