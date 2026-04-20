# Admin Billing Navigation Design

## Goal

Separate customer operational billing from tenant platform billing so admins can find daily ISP billing work faster.

## Navigation Decision

Customer billing belongs in the main `Billing` sidebar section. Tenant platform subscription, plans, and SaaS payment history belong under `Settings > Billing & Plan`.

## Scope

This first implementation changes navigation and IA safely:

- Remove `Subscription` from the main Billing section.
- Make the main Billing section focus on customer billing:
  - `Billing` -> customer invoice list (`/admin/invoices`)
  - `Collections` -> billing logs/collection (`/admin/invoices/collection`)
- Add a `Billing & Plan` category to Settings that explains tenant plan/subscription and links to the existing tenant subscription page.
- Keep `/admin/subscription` alive as the existing tenant plan detail route to avoid breaking old links.

## Future Follow-Up

The existing `/admin/subscription` page is large and should be extracted into a reusable `TenantBillingPlanPanel.svelte` before fully embedding it inside Settings. That refactor should be separate so the navigation change remains low-risk.
