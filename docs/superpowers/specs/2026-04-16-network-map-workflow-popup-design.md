# Network Map Workflow Popup Design

## Goal

Polish customer/service popups in the admin network map so operators can quickly understand the service and open the related customer record.

## Selected Direction

Use the **Workflow First** direction. The popup should prioritize operator action over raw metadata density.

Primary action:

- `Open Customer`

Secondary actions:

- `Open Service` or subscription detail when a stable destination exists.
- `Connect` for topology editing.
- `Close` as a utility action.

## UI Structure

The service popup keeps the existing dark map overlay language, but improves hierarchy:

- Header with entity type, service name, customer name, and status badge.
- Context strip with lifecycle/provisioning copy, for example `Subscription active • PPPoE synced`.
- Two quick-fact cards for `Customer` and `Account`.
- Compact detail rows for package, service type, and optional router/profile metadata.
- Action row with `Open Customer` as the primary button.

## Data And Routing

Service nodes should use existing map node metadata:

- `customer_id` for `Open Customer`.
- `service_id`, `subscription_id`, or existing service metadata for display and future deep links.
- `pppoe_username`, `package_name`, `service_type`, and `customer_name` for quick facts.

When `customer_id` is missing, the popup should not render `Open Customer`; it should still support `Connect`.

## Testing

Unit tests should lock the service popup model:

- Service popups expose `Open Customer` as the first action when `customer_id` exists.
- Service popups retain `Connect` after customer navigation.
- Service popup summary/details include customer, account, and package context.
