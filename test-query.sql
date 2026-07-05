EXPLAIN ANALYZE
SELECT COUNT(*)
FROM mikrotik_logs l
WHERE l.tenant_id = '018f3a3a-3333-7777-8888-999999999999'
  AND (NULL::text IS NULL OR l.router_id = NULL)
  AND (NULL::text IS NULL OR l.level = NULL)
  AND (NULL::text IS NULL OR l.topics ILIKE '%' || NULL || '%')
  AND ('' = '' OR l.message ILIKE '%' || '' || '%')
  AND (7::int4 IS NULL OR EXTRACT(MONTH FROM l.logged_at) = 7)
  AND (2026::int4 IS NULL OR EXTRACT(YEAR FROM l.logged_at) = 2026);
