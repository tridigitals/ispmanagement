ALTER TABLE isp_packages
  DROP CONSTRAINT IF EXISTS isp_packages_price_monthly_nonneg;

ALTER TABLE isp_packages
  DROP COLUMN IF EXISTS price_monthly;