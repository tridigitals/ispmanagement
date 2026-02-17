ALTER TABLE isp_packages
  ADD COLUMN price_yearly numeric(12,2) NOT NULL DEFAULT 0;

ALTER TABLE isp_packages
  ADD CONSTRAINT isp_packages_price_yearly_nonneg CHECK (price_yearly >= 0);