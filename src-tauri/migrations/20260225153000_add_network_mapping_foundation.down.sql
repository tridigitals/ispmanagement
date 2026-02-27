-- rollback network mapping foundation

DROP TRIGGER IF EXISTS trg_service_zones_set_updated_at ON public.service_zones;
DROP TRIGGER IF EXISTS trg_network_links_set_updated_at ON public.network_links;
DROP TRIGGER IF EXISTS trg_network_nodes_set_updated_at ON public.network_nodes;

DROP TABLE IF EXISTS public.zone_node_bindings;
DROP TABLE IF EXISTS public.service_zones;
DROP TABLE IF EXISTS public.network_links;
DROP TABLE IF EXISTS public.network_nodes;

DROP FUNCTION IF EXISTS public.tg_set_updated_at();
