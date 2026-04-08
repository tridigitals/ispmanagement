# Admin Dashboard RBAC Adaptive Design

## Goal
- Redesign `/[tenant]/admin` so it behaves like an adaptive operations dashboard instead of a generic admin landing page.
- Show only cards, charts, summaries, and actions that are relevant to the current tenant role and granted permissions.
- Make the dashboard useful for daily work for Owner/Admin, Technician, Customer Service, NOC, and other granular tenant roles.

## Problem
The current admin dashboard is too generic:
- it shows the same high-level cards regardless of whether the user is an admin, technician, CS, or NOC
- it emphasizes team/settings/subscription even for roles that primarily work on installations, incidents, PPPoE, or billing follow-up
- its quick actions are not strongly aligned to the user's real permission surface
- it does not visually communicate read-only versus action-capable contexts

As RBAC becomes more granular, the dashboard must become equally granular so the first screen after login feels correct for the role.

## Product Direction
Use a single dashboard shell with adaptive role-aware content.

That means:
- one `/admin` route remains the entry point
- sections inside the page are assembled dynamically from permission-aware widgets
- each widget is rendered only when the user has the required permission(s)
- each widget should link to the actual work page for that capability
- the dashboard should prefer small dense operational summaries over large generic charts

## Target UX
The dashboard should answer three questions immediately:
- What can I work on?
- What needs attention today?
- Where do I go next?

### Top-level layout
1. `Role-aware KPI cards`
2. `My Focus Today`
3. `Quick Actions`
4. Optional `Mini charts / trend strips` only when useful and data exists

### UX principles
- No empty admin chrome for low-privilege roles
- No cards for capabilities the user cannot access
- No data fetches for widgets the user cannot see
- Read-only roles should still get useful summaries, not just blank states
- Action-heavy roles should land on operational cards first
- Keep desktop information-dense but still readable on mobile

## Recommended Information Model
### 1. Role-aware KPI cards
The first row should show 2-4 cards depending on permission surface.

Examples:

#### Owner / Admin
- Team members
- Customer count
- Billing overview
- Plan / subscription status

#### Technician
- Active installation work orders
- Pending PPPoE/router actions
- Assigned customers or active service visits
- Optional PPPoE sync status card

#### Customer Service
- Total customers
- Unpaid / pending invoices
- Pending support follow-ups
- New onboarding / invite activity

#### NOC
- Open incidents
- Active alerts
- Router health / monitored routers
- Network logs or degraded devices summary

### 2. My Focus Today
This is the main adaptive section and should feel role-first.

Rules:
- maximum 2-4 focus cards
- each card should include number, short status text, and a CTA
- this section should carry the strongest visual emphasis after KPI cards

Examples:

#### Technician focus
- `Installations waiting for action`
- `PPPoE accounts ready to apply`
- `Customers in read-only context`

#### CS focus
- `Invoices waiting payment follow-up`
- `Customers needing activation follow-up`
- `Support tickets awaiting reply`

#### NOC focus
- `Incidents breaching SLA`
- `Alerts needing triage`
- `Routers requiring attention`

#### Owner/Admin focus
- `Cross-team exceptions`
- `Billing risk`
- `Operational workload`

### 3. Quick Actions
Quick Actions should become permission-aware navigation shortcuts instead of a static menu duplicate.

Rules:
- show only actions the user can actually execute
- prefer “do work” links over “browse settings” links
- maximum 4-6 cards

Examples:
- Technician: Installations, PPPoE, Customers
- CS: Customers, Invoices, Support
- NOC: NOC, Alerts, Incidents, Routers
- Owner/Admin: Team, Roles, Settings, Billing, Customers

### 4. Mini charts and trend strips
Charts are optional and should only appear when they support a real decision.

Recommended:
- small sparkline/trend tiles
- compact bar strips for status distributions
- no large analytics dashboard in phase one

Good chart candidates:
- billing status distribution for billing-capable roles
- work order status distribution for work-order roles
- incident severity distribution for network roles

Avoid:
- large empty chart placeholders
- charts for users who cannot act on the underlying data

## RBAC Rendering Contract
Each widget must declare:
- `id`
- `title`
- required permissions
- optional fetch function
- click destination
- display priority

Rendering rules:
- widget is hidden if permission requirement fails
- widget data is not fetched if widget is hidden
- widget empty states are contextual to the role
- widget CTA routes must already be permitted by route guard logic

## Data Sources
Prefer existing APIs first. Do not create a heavy dashboard aggregation backend in phase one unless existing endpoints are insufficient.

Likely reusable sources:
- `team.list()`
- `settings.getAll()`
- `api.plans.getSubscriptionDetails(...)`
- customer list / customer summary endpoints
- billing / invoice list endpoints
- work order / installation endpoints
- alerts / incidents / router list endpoints
- PPPoE / router sync endpoints where relevant

If multiple widgets need the same underlying domain data:
- fetch once per domain
- derive multiple cards locally where reasonable

## Visual Design Direction
Keep the current visual language but make the hierarchy more intentional.

Design guidance:
- stronger separation between summary cards and work cards
- richer card grouping and spacing
- more purposeful color ownership by domain:
  - green for operations ready / active work
  - amber for attention needed
  - cyan/blue for infrastructure
  - indigo for billing/subscription
- use mini status strips, small sparklines, and badges instead of oversized charts
- low-permission dashboards should feel concise, not empty

## Accessibility and Responsiveness
- all cards remain keyboard navigable
- action cards should preserve button semantics
- summary-only cards should not appear clickable unless they navigate
- stacked layout on smaller screens should preserve widget priority order
- metrics and badges should keep sufficient contrast in dark theme

## Implementation Shape
### Frontend page
Refactor `/[tenant]/(app)/admin/+page.svelte` to:
- build a permission-aware widget registry
- compute visible widgets based on `$can(...)`
- fetch only relevant domain data
- render grouped sections from filtered widget arrays

### Suggested section boundaries
- `primaryStats`
- `focusCards`
- `quickActions`
- `trendCards`

### Recommended internal helpers
- capability predicates derived from `$can(...)`
- widget builder helpers
- small formatter helpers for counts/status labels

## Examples of Final Behavior
### Technician
- sees no team/settings emphasis
- sees work-order and PPPoE summaries
- sees customer access as read-only operational context
- quick actions prioritize installations, PPPoE, and customers

### Customer Service
- sees customer, billing, and support summaries
- does not see network-heavy cards
- quick actions prioritize customers, invoices, support

### NOC
- sees network alerts/incidents/logs/routers summaries
- does not see billing/team/settings emphasis except where explicitly allowed
- quick actions prioritize NOC tooling

### Owner/Admin
- sees broader business + operational snapshot
- keeps access to team, roles, settings, billing, customers

## Non-goals
- No new analytics warehouse or complex backend dashboard API in phase one
- No role-name hardcoding as the primary logic source; permissions remain primary
- No full dashboard personalization system
- No drag-and-drop widget customization
- No large custom charting subsystem

## Testing Strategy
### Frontend
- dashboard only renders widgets whose permissions are granted
- hidden widgets do not trigger their data fetches
- technician dashboard does not show admin-only cards
- NOC dashboard prioritizes network widgets
- billing widgets stay hidden without billing permission
- `/admin` remains useful with minimal permission surfaces

### Backend / integration assumptions
- existing endpoint permissions continue to enforce access server-side
- dashboard links should never lead to pages denied by route guards

## Acceptance Criteria
- `/admin` feels materially different for technician, CS, NOC, and owner/admin users
- widgets are permission-aware and fetch only relevant data
- admin-only summary cards no longer dominate low-privilege dashboards
- quick actions match the actual allowed work surface
- at least one compact trend/summary visualization is shown where useful
- route links from cards align with existing RBAC guards
