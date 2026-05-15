# FTTH ODP Occupancy And Topology Links

## Goal

Improve FTTH asset visibility on `/admin/network/map` and ODP operational insight by:

- showing `total / used / available` ports for ODP
- deriving ODP usage from both terminal assets and direct customer attachment data
- drawing automatic topology links between FTTH assets and customer markers
- keeping manual topology links available for cases not covered by asset relations

## Existing Constraints

- FTTH assets are stored in `network_assets`
- ODP occupancy currently counts only `ONT/ONU` children via `parent_asset_id`
- topology map renders FTTH assets as a separate overlay from `network_nodes`
- customer markers already exist as synced `customer_premise` / `customer_endpoint` nodes
- there is no dedicated persisted table yet for asset-to-asset map links

## Data Model Strategy

### ODP occupancy

For `asset_type = odp`, port usage is derived from unique endpoint attachments:

1. terminal assets (`ont`, `onu`) whose `parent_asset_id = odp.id`
2. direct customer/service attachment rows under the same ODP when customer/location data exists even without ONT/ONU

De-duplication priority:

1. `location_id`
2. `customer_id`
3. asset id

This prevents double counting when both a terminal asset and a fallback customer attachment represent the same endpoint.

### Auto topology links

The map derives non-persisted overlay links from existing records:

1. parent-child FTTH asset relations
   - e.g. `ODC -> ODP`, `OLT -> ODC`, `ODP -> FAT`
2. customer-facing links from ODP to customer marker
   - resolved from terminal/direct attachment rows to synced customer nodes

These links are visual overlays only. They do not replace manual topology links in `network_mapping`.

## UI Behavior

### Asset page

- ODP rows continue to show occupancy label
- occupancy now reflects hybrid endpoint counting instead of ONT/ONU-only counting

### Map page

- ODP popup shows:
  - total port capacity
  - ports used
  - ports available
- FTTH asset overlay draws automatic dashed lines:
  - asset-to-asset parent relation
  - ODP-to-customer fallback/service relation
- auto links follow the existing map visibility model:
  - hidden when topology assets are hidden
  - hidden when links are hidden

## Implementation Scope

1. extend occupancy util to support hybrid endpoint counting
2. enrich topology asset rows with relation and occupancy fields
3. add derived FTTH auto-link overlay source/layer
4. update map popup to show ODP occupancy
5. add tests for occupancy and auto-link generation

## Out Of Scope

- new persisted asset-link database tables
- interactive connect/draw UI for FTTH asset markers
- port-level numbered fiber splice management

## Verification

- unit tests for occupancy de-duplication and auto-link derivation
- `npm run check`
- browser verification on `info@xtrabit.com` tenant map view
