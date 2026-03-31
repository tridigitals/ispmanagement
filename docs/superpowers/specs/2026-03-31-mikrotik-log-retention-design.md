# Mikrotik Log Retention Design (2026-03-31)

## Background / Current State
- Current implementation enforces a hard storage cap of 5000 logs per router during sync in `sync_logs_for_router()` (`src-tauri/src/services/mikrotik_service.rs`, around line 719).
- Pruning currently uses a count-based query pattern around `OFFSET 5000` (`src-tauri/src/services/mikrotik_service.rs`, around line 839).
- API/UI pagination behavior is already in place and should remain unchanged.

## Goals / Non-goals
### Goals
- Replace count-based hard cap with time-based retention.
- Default retention window is **90 days**.
- Retention window is configurable via environment variable `MIKROTIK_LOG_RETENTION_DAYS`.
- Add sane config validation with fallback to 90 days.
- Keep existing sync/upsert flow.
- Keep existing API/UI pagination behavior.
- Remove any 5000-row storage cap behavior.
- Ensure pruning failure causes sync failure (no false success).
- Address indexing/performance expectations for larger retained datasets.

### Non-goals
- No changes to API contract or UI pagination semantics.
- No redesign of sync scheduling or ingestion architecture.
- No new archival tier beyond in-database retention window pruning.

## Architecture Changes
1. Replace count-based prune step with time-based prune step scoped per router.
2. Compute retention cutoff as: `now_utc - retention_days`.
3. Delete logs older than cutoff (router-scoped).
4. Keep sync/upsert stages unchanged except for replacing prune criteria.
5. Remove logic that depends on fixed `5000` threshold for storage trimming.

## Data Flow
1. Read retention configuration (`MIKROTIK_LOG_RETENTION_DAYS`) at runtime using existing config-loading path.
2. Validate value:
   - If unset: use 90.
   - If parse fails: use 90.
   - If value is non-positive: use 90.
   - If value is greater than 3650: use 90.
3. Run normal router log fetch + upsert flow.
4. Execute prune query deleting records where log timestamp `< cutoff` for the target router.
5. Return success only if all phases (fetch/upsert/prune) succeed.

## Error Handling
- Prune is part of sync transaction outcome semantics.
- If prune step fails, overall sync operation must fail.
- Failure must be surfaced through existing error propagation path; do not report successful sync when prune fails.

## Performance / Indexing
- Expected dataset growth increases under 90-day window versus hard 5000 cap, so prune/read paths must be index-supported.
- Validate (and add if missing) indexes that support:
  - Router-scoped timestamp pruning (`router_id`, `time`).
  - Router-scoped time-ordered pagination (`router_id`, `time DESC`, with stable tie-breaker if needed).
- Keep prune query router-scoped and timestamp-bounded to avoid full-table scans.
- Confirm query planner uses indexes for prune and paginated reads under representative data volume.

## Config Contract (env + defaults + validation)
- Variable: `MIKROTIK_LOG_RETENTION_DAYS`
- Type: integer days
- Default: `90`
- Validation / fallback:
  - Missing => 90
  - Non-integer => 90
  - `<= 0` => 90
  - `> 3650` => 90
- Effective retention value must be deterministic and logged/debug-visible through existing config observability patterns.

## Testing Strategy
- Unit tests for config parsing/validation:
  - unset, invalid string, zero, negative, too-large, valid custom value.
- Integration tests for sync/prune behavior:
  - records older than cutoff are removed;
  - records within retention remain;
  - no fixed-count cap behavior remains.
- Failure-path test:
  - simulated prune failure causes sync failure result.
- Performance verification:
  - explain/query-plan checks (or equivalent) confirm index usage on prune and paginated list queries.

## Rollout
1. Deploy with default config (no env override) to activate 90-day retention.
2. Optionally set `MIKROTIK_LOG_RETENTION_DAYS` per environment if policy differs.
3. Monitor sync error rate and prune latency during initial rollout window.
4. Validate pagination behavior remains unchanged from client perspective.

## Risks / Mitigations
- **Risk:** Larger retained volume may increase query latency.
  - **Mitigation:** Ensure composite indexes for router/time, validate plans, monitor latency.
- **Risk:** Misconfigured env value causing unexpected retention.
  - **Mitigation:** Strict validation with fallback to 90 and clear effective-config logging.
- **Risk:** Silent prune failure could mislead operators.
  - **Mitigation:** Hard-fail sync when prune fails, preserving truthful operation status.

## Acceptance Criteria
- Count-based `5000` storage cap logic is removed from retention behavior.
- Retention is time-based with default 90 days.
- `MIKROTIK_LOG_RETENTION_DAYS` is supported with documented validation/fallback to 90.
- Sync/upsert flow remains intact; API/UI pagination behavior unchanged.
- Prune failure causes overall sync failure.
- Indexing strategy for router/time prune and pagination is validated/documented for 90-day window.
- Tests cover config parsing, prune correctness, absence of count cap, and prune-failure propagation.
