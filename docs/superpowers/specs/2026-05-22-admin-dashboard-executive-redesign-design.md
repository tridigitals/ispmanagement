# Admin Dashboard Executive Redesign Design

## Summary
Redesign `/[tenant]/admin` into a more professional, dark-first executive dashboard that feels production-ready and intentionally designed, not like a starter template. The page should stay lightweight, readable, and actionable for admins while avoiding decorative excess.

## Goals
- Make the dashboard feel more premium and mature.
- Reduce the “generic dashboard card grid” look.
- Keep the page airy and easier to scan at a glance.
- Preserve the current data model and click flows.
- Add lightweight visual trend treatment where it improves comprehension.

## Non-goals
- No change to backend data fetching.
- No change to dashboard navigation destinations.
- No heavy charting library.
- No bright gradients, noisy visual effects, or marketing-style hero treatment.

## Approved Direction
The approved direction is:
- `lapang`
- `executive board`
- `dark-first`
- `minimal text`
- `lightweight bars/progress/donut visuals only when useful`

## Layout
The page should be structured into four layers:

1. `Executive masthead`
- strong title
- concise subtitle
- compact role/context pill
- refresh action
- timestamp as metadata

2. `Primary KPI row`
- high-value cards
- shorter height
- stronger number hierarchy
- quieter labels and badges

3. `Focus band`
- still the main actionable area
- more editorial layout and cleaner CTA treatment
- sharper hierarchy than the KPI row

4. `Decision layer`
- `Quick Actions` on one side
- `Compact Trends` on the other
- trends should use small compositional visuals rather than big charts

## Trend Strategy
Prefer:
- compact distribution bars
- mini progress compositions
- at most one small donut-like visualization if it helps summarize a category

Avoid:
- large charts
- decorative analytics widgets
- BI-dashboard complexity

## Visual System
- charcoal/slate layered backgrounds
- subtle borders
- consistent radii
- restrained shadows
- bright-but-controlled typography
- accent colors reserved for state and emphasis

## Tone
The dashboard should feel:
- executive
- calm
- operationally credible
- productized

It should not feel:
- playful
- experimental
- over-designed
- “AI dashboard” or “vibe coding”

## Scope
Implementation is limited to:
- `/src/routes/(app)/admin/+page.svelte`
- tests that guard the new presentation structure

The redesign should remain a presentation-layer change.
