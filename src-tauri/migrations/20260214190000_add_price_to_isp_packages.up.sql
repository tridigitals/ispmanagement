ALTER TABLE isp_packages
  ADD COLUMN price_monthly numeric(12,2) NOT NULL DEFAULT 0;

ALTER TABLE isp_packages
  ADD CONSTRAINT isp_packages_price_monthly_nonneg CHECK (price_monthly >= 0);