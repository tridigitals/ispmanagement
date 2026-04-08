# Network Map Topology Workspace Design

## Summary
Redesign `/[tenant]/admin/network/map` from a topology CRUD-heavy page into a map-first operational workspace for both technicians and NOC operators.

The new page should make it easy to:
- monitor route and segment health
- investigate incidents from either network assets or customer/service symptoms
- understand customer and service impact quickly
- act from map context without jumping across many pages
- keep topology management available, but clearly secondary to monitoring and investigation

## Goals / Non-goals
### Goals
- Make the map canvas the primary workspace surface.
- Support balanced use by `technician`, `NOC/operator`, and `admin/manage` roles.
- Elevate `customer/service impact` to a first-class concern alongside asset topology.
- Replace static CRUD-first interactions with map popups and a smart inspector flow.
- Provide a clear incident investigation path:
  - from `node/link` to impacted services/customers
  - from `customer/service` to serving path and likely fault points
- Keep topology editing available for allowed roles through an explicit manage mode.
- Improve information density and clarity without making the page feel like a crowded control room.

### Non-goals
- No full GIS or external geospatial analytics system in this phase.
- No new heavy charting dependency for the map page.
- No attempt to solve every live telemetry integration in this phase.
- No replacement of existing customer, service, PPPoE, work-order, or incident detail pages.
- No duplication of full CRUD forms inside the popup surface.

## Product Positioning
The page should become a `Topology Workspace`, not just a `Network Mapping Admin Page`.

That means:
- monitoring is the default mode
- inspection and investigation are the primary click flows
- management/editing is an explicit secondary mode

The mental model should be:
1. Monitor
2. Inspect
3. Investigate
4. Act
5. Manage

## Primary Users
### Technician
Primary needs:
- locate the customer/service problem quickly
- see the nearest and serving network assets
- understand likely fault path
- pivot to customer, service, work order, or navigation quickly

### NOC / Operator
Primary needs:
- identify degraded or down network segments
- understand downstream impact
- trace likely fault points
- create or route operational follow-up quickly

### Admin / Manage
Primary needs:
- retain all monitor and investigate flows
- enter explicit topology management mode when needed
- manage nodes, links, zones, and topology sync safely

## Design Direction
Recommended direction: `Map First + Smart Inspector`

The page should feel visually closer to an operational workspace than a settings console:
- large map canvas
- concise top insight strip
- powerful unified search
- floating map controls
- quick operational popup on object click
- right-side smart inspector for deeper detail

## Information Architecture
### Layout
The page should be composed of five main regions:

1. `Top insight strip`
- operational summary cards
- concise, high-signal, role-aware

2. `Unified search and quick mode bar`
- one search entry point across assets, customers, services, and zones
- mode chips for fast context switching

3. `Main map canvas`
- dominant visual area
- large enough to feel like the real workspace

4. `Floating map controls`
- layers
- view mode
- trace tools
- role-aware manage actions

5. `Single smart inspector`
- contextual right-side panel
- changes based on selected object or investigation flow

Optional:
6. `Secondary bottom panel`
- impacted services/customers
- trace path steps
- recent operational activity
- manage tables or bulk topology views when explicitly requested

## Top Insight Strip
The top cards should not be simple inventory counters only. They should be split between `operational risk` and `network inventory`.

Recommended card families:

### Operational cards
- `Nodes at risk`
- `Links degraded`
- `Impacted services`
- `Critical areas`
- `Active field work`

### Inventory cards
- `Routers`
- `OLT`
- `ODC`
- `ODP`
- `Homepass`
- `Active services`

### Behavior
- Support `global summary` and `viewport summary`.
- Role-aware ordering:
  - technicians see service and field relevance first
  - NOC sees issue severity and impact first
  - manage roles can see broader asset totals

## Unified Search
Search should query and group results by:
- network assets
- services
- customers
- zones/areas
- addresses or location labels when available

Selecting a result should:
- focus the map
- highlight the selected feature
- open a popup immediately
- optionally open the smart inspector for deeper context

## Quick Mode Chips
Recommended mode chips:
- `All`
- `Issues`
- `Customers`
- `Services`
- `Topology`
- `Field Mode`

These should feel faster and lighter than multi-select filter forms.

Advanced filters can remain available in a secondary control surface.

## Popup Design
The popup is a `decision surface`, not a generic tooltip.

Every popup should answer:
- what is this object
- what is its condition
- what is the impact
- what can I do next

### Shared popup structure
1. Identity
2. Health / status
3. Impact summary
4. Quick actions

The popup should stay short and scannable, with a clear action row and an `Open inspector` escape hatch.

### Node popup
Recommended content:
- name
- node type
- health/status
- location/zone
- upstream/downstream short summary
- active or degraded link count
- impacted services count
- last heartbeat/update if available

Quick actions:
- `Trace path`
- `View impacted services`
- `Open inspector`
- `Open related work orders`
- `Open router/device`
- `Edit topology` for manage roles

### Link popup
Recommended content:
- link name
- endpoint A -> B
- status
- health score
- capacity/utilization
- latency/loss/signal if available
- impacted services count

Quick actions:
- `Trace downstream impact`
- `View affected services`
- `Open inspector`
- `Create incident`
- `Create work order`
- `Edit link`

### Customer / service popup
This popup should prioritize `service operational state` over customer profile details.

Recommended service content:
- service label / customer name
- service state: `online`, `offline`, `suspended`, `isolated`, `unknown`
- package
- PPPoE username
- IP / ONU / CPE info when available
- serving node path summary: ODP / ODC / OLT / upstream node
- signal/session/uptime metrics when available
- overdue or suspended risk badge when operationally relevant

Recommended customer content:
- customer ID
- short address
- phone
- area/zone

Quick actions:
- `Open customer`
- `Open service`
- `Trace route`
- `Open PPPoE/session`
- `Open ticket`
- `Open work order`
- `Navigate`

### Zone popup
Recommended content:
- zone name
- type
- status
- asset count
- service/customer count
- issue density summary

Quick actions:
- `Inspect area`
- `View services in area`
- `Create work order in area`
- `Edit zone`

## Single Smart Inspector
The right panel should be a single adaptive inspector, not multiple unrelated side panels.

### Default state
When nothing is selected, the inspector should show:
- workspace summary
- issue highlights
- recent selections
- shortcuts into key investigation flows

### Object-specific modes
#### Node inspector
- overview
- health
- upstream/downstream
- connected links
- impacted services
- related incidents/work orders
- topology metadata

#### Link inspector
- endpoints
- health metrics
- capacity/utilization
- affected path
- affected services
- degradation history if available

#### Service inspector
- service health
- customer summary
- package/profile/PPPoE/session
- serving path
- recent issue or visit context
- billing risk snapshot only when operationally relevant

#### Zone inspector
- area overview
- asset count
- service density
- active issues
- area-focused actions

### Investigation mode
This is the most important deeper state.

When the user chooses `Trace` or `Investigate`, the inspector should switch into an investigation view showing:
- root object
- highlighted upstream/downstream path
- likely fault candidate
- impacted services count
- impacted customer/service list
- related incidents/work orders
- suggested next actions

## Floating Controls
The right-side floating controls should stay compact and tactile.

Recommended groups:

### Layers
- nodes
- links
- zones
- services
- customers
- routers
- incidents

### Map style
- standard
- satellite
- hybrid

### Trace tools
- trace upstream
- trace downstream
- impact mode

### Focus filters
- only issues
- only offline services
- only degraded links
- only active work orders
- only selected zone

### Manage tools
Visible only to roles with manage capability:
- add node
- add link
- add zone
- sync topology assets
- open manage mode

## Role-Adaptive Behavior
The layout should stay consistent across roles, but defaults and emphasis should change.

### Technician defaults
- default mode favors `Services` or `Field`
- customer/service markers more prominent
- work-order and address context closer to top
- manage tools minimized or hidden

### NOC defaults
- default mode favors `Issues`
- node and link health more prominent
- impact tracing emphasized
- incident-related actions surfaced early

### Manage defaults
- broader overview mode allowed
- edit/manage controls available
- monitoring remains primary, manage tools secondary

## State Design
### Initial loading
- map skeleton remains visible
- summary cards use skeletons
- search and frame structure appear immediately

### Empty / low-data state
If topology data is minimal:
- keep map visible
- explain whether sync or setup is missing
- show role-appropriate CTA:
  - `Sync assets`
  - `Add first node`
  - `Create first zone`

### Selected object state
- highlight the selected object
- open popup
- sync inspector

### Investigation state
- highlight route/path
- dim irrelevant layers
- show impact chain and likely fault point

### Impact list state
- open customer/service list in bottom panel or inspector
- allow click-to-focus back into map

### Manage state
- explicit enter/exit manage mode
- never mix edit affordances too aggressively into monitor mode

### Partial failure state
If one data source fails:
- page should keep working
- show precise warnings such as:
  - router overlay unavailable
  - live service metrics unavailable
  - topology sync stale

### Freshness state
Show lightweight freshness messaging:
- `Updated 12s ago`
- `Refreshing...`
- `Partial live data`

## Mobile / Tablet Behavior
Desktop is the primary target, but the page must remain usable on smaller screens.

Recommended mobile/tablet behavior:
- top cards become horizontal scroll
- smart inspector becomes drawer or bottom sheet
- popup content is shortened
- floating controls are reduced
- map remains the dominant surface

## Data and Integration Expectations
The redesign assumes the page can draw from existing or near-term data sources such as:
- network topology nodes, links, zones
- router inventory overlays when permitted
- customer/service geolocation or assigned network path
- PPPoE/service state when available
- work orders / installations related to selected customers or areas
- incidents / alerts when integrated

Phase one can ship with partial live metrics as long as the UI clearly distinguishes:
- topology structure
- cached/snapshotted data
- live operational status

## UX Principles
- Map first
- Popup first interaction
- Inspector for depth
- Investigation over CRUD
- Customer/service impact must be visible early
- Editing is explicit, not ambient
- Role-adaptive emphasis without fragmenting the page into separate products

## Recommended Implementation Order
1. Redesign top summary, search, and floating controls.
2. Redesign object popups.
3. Build the smart inspector.
4. Add investigation mode and impact tracing UI.
5. Move existing manager tables into secondary/manage mode.
6. Polish role-adaptive defaults and partial-data states.

## Acceptance Criteria
- The page feels like a map-first operational workspace instead of a topology CRUD page.
- Technicians can investigate customer/service problems from map context quickly.
- NOC operators can trace issues from assets to downstream impact quickly.
- Popups provide clear status, impact, and next actions.
- A single smart inspector supports deeper investigation without constant page switching.
- Customer/service impact is first-class in the page design.
- Manage-mode tools remain available without overwhelming monitor-mode users.
- The page remains usable with partial data and across desktop/tablet/mobile breakpoints.
