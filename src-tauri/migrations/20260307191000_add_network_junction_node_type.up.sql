ALTER TABLE public.network_nodes
  DROP CONSTRAINT IF EXISTS chk_network_nodes_type;

ALTER TABLE public.network_nodes
  ADD CONSTRAINT chk_network_nodes_type
  CHECK (
    node_type IN (
      'core',
      'pop',
      'olt',
      'router',
      'switch',
      'tower',
      'ap',
      'odc',
      'odp',
      'splitter',
      'junction',
      'customer_endpoint',
      'customer_premise'
    )
  );
