UPDATE public.network_links
SET status = 'maintenance'
WHERE status IN ('planning', 'retired');

ALTER TABLE public.network_links
  DROP CONSTRAINT IF EXISTS chk_network_links_status;

ALTER TABLE public.network_links
  ADD CONSTRAINT chk_network_links_status
    CHECK (status IN ('up', 'down', 'degraded', 'maintenance'));

