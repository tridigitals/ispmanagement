use crate::models::MixradiusImportConflictState;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MixradiusMapperDecision {
    pub state: MixradiusImportConflictState,
    pub action: String,
    pub source_kind: String,
    pub source_ref: String,
    pub target_kind: Option<String>,
    pub target_id: Option<String>,
    pub display_name: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StagedPlan {
    pub source_ref: String,
    pub plan_name: String,
    pub price: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExistingPackage {
    pub id: String,
    pub name: String,
    pub price_monthly: f64,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StagedNas {
    pub source_ref: String,
    pub nas_name: String,
    pub nas_ip_or_cidr: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExistingRouter {
    pub id: String,
    pub name: String,
    pub host: String,
    pub identity: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MappingOverride {
    pub source_ref: String,
    pub target_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StagedCustomer {
    pub source_ref: String,
    pub member_id: String,
    pub username: Option<String>,
    pub fullname: Option<String>,
    pub email: Option<String>,
    pub phonenumber: Option<String>,
    pub trx_status: Option<String>,
    pub expired_on: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExistingCustomer {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MixradiusExternalRef {
    pub entity_type: String,
    pub entity_id: String,
    pub source_ref: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExistingPppoeAccount {
    pub id: String,
    pub router_id: String,
    pub username: String,
    pub customer_id: String,
    pub disabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MixradiusPppoeAction {
    New,
    Update,
    Same,
    Blocked,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MixradiusPppoeDecision {
    pub state: MixradiusImportConflictState,
    pub action: MixradiusPppoeAction,
    pub target_id: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MixradiusLifecyclePreview {
    pub status: String,
    pub warnings: Vec<String>,
    pub requires_review: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CustomerUpdatePatch {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub warnings: Vec<String>,
}

pub struct MixradiusImportMapperPolicy;

impl MixradiusImportMapperPolicy {
    pub fn normalize_subscription_lifecycle(
        trx_status: Option<&str>,
        expired_on: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> MixradiusLifecyclePreview {
        let status = trx_status.unwrap_or_default().trim().to_ascii_uppercase();

        match status.as_str() {
            "PAID" => MixradiusLifecyclePreview {
                status: "active".into(),
                warnings: vec![],
                requires_review: false,
            },
            "POSTPAID" => MixradiusLifecyclePreview {
                status: "active".into(),
                warnings: vec![],
                requires_review: false,
            },
            "UNPAID" if expired_on.is_some_and(|expired_at| expired_at < now) => {
                MixradiusLifecyclePreview {
                    status: "suspended".into(),
                    warnings: vec![],
                    requires_review: false,
                }
            }
            "UNPAID" => MixradiusLifecyclePreview {
                status: "active".into(),
                warnings: vec![
                    "Tagihan MixRadius belum lunas, tetapi masa aktif belum habis; preview menjaga subscription tetap active dan menandai billing untuk dicek."
                        .into(),
                ],
                requires_review: false,
            },
            "PENDING" => MixradiusLifecyclePreview {
                status: "pending_installation".into(),
                warnings: vec![
                    "Status MixRadius PENDING perlu review sebelum masuk lifecycle billing ISP Management."
                        .into(),
                ],
                requires_review: true,
            },
            "" => MixradiusLifecyclePreview {
                status: "pending_installation".into(),
                warnings: vec!["Status transaksi MixRadius kosong; perlu review.".into()],
                requires_review: true,
            },
            other => MixradiusLifecyclePreview {
                status: "pending_installation".into(),
                warnings: vec![format!(
                    "Status transaksi MixRadius `{other}` belum dipetakan; perlu review."
                )],
                requires_review: true,
            },
        }
    }
}

pub fn resolve_package_mapping(
    plan: &StagedPlan,
    existing_packages: &[ExistingPackage],
    overrides: &[MappingOverride],
) -> MixradiusMapperDecision {
    if let Some(override_mapping) = overrides
        .iter()
        .find(|mapping| mapping.source_ref == plan.source_ref)
    {
        return decision(
            MixradiusImportConflictState::AutoMatched,
            "override",
            "plan",
            &plan.source_ref,
            Some("package"),
            Some(&override_mapping.target_id),
            Some(&plan.plan_name),
            Some("Package dipilih manual dari mapping override."),
        );
    }

    let normalized_plan_name = normalize_key(&plan.plan_name);
    if let Some(package) = existing_packages
        .iter()
        .find(|package| normalize_key(&package.name) == normalized_plan_name)
    {
        let notes = if package.is_active {
            "Package existing dipakai ulang berdasarkan nama exact."
        } else {
            "Package existing cocok nama, tetapi sedang nonaktif; perlu dicek sebelum execute."
        };
        return decision(
            if package.is_active {
                MixradiusImportConflictState::AutoMatched
            } else {
                MixradiusImportConflictState::NeedsReview
            },
            "reuse",
            "plan",
            &plan.source_ref,
            Some("package"),
            Some(&package.id),
            Some(&plan.plan_name),
            Some(notes),
        );
    }

    decision(
        MixradiusImportConflictState::NeedsReview,
        "create",
        "plan",
        &plan.source_ref,
        Some("package"),
        None,
        Some(&plan.plan_name),
        Some("Package belum ada; preview akan membuat package baru bila disetujui."),
    )
}

pub fn resolve_router_mapping(
    nas: &StagedNas,
    existing_routers: &[ExistingRouter],
    overrides: &[MappingOverride],
) -> MixradiusMapperDecision {
    if let Some(override_mapping) = overrides
        .iter()
        .find(|mapping| mapping.source_ref == nas.source_ref)
    {
        return decision(
            MixradiusImportConflictState::AutoMatched,
            "override",
            "nas",
            &nas.source_ref,
            Some("router"),
            Some(&override_mapping.target_id),
            Some(&nas.nas_name),
            Some("Router dipilih manual dari mapping override."),
        );
    }

    let nas_name = normalize_key(&nas.nas_name);
    let nas_host = normalize_host(&nas.nas_ip_or_cidr);
    let mut matches = existing_routers.iter().filter(|router| {
        router.enabled
            && (normalize_key(&router.name) == nas_name
                || normalize_host(&router.host) == nas_host
                || router
                    .identity
                    .as_deref()
                    .map(normalize_key)
                    .is_some_and(|identity| identity == nas_name))
    });

    let first_match = matches.next();
    let second_match = matches.next();
    match (first_match, second_match) {
        (Some(router), None) => decision(
            MixradiusImportConflictState::AutoMatched,
            "reuse",
            "nas",
            &nas.source_ref,
            Some("router"),
            Some(&router.id),
            Some(&nas.nas_name),
            Some("Router existing cocok berdasarkan nama, identity, atau host."),
        ),
        (Some(_), Some(_)) => decision(
            MixradiusImportConflictState::NeedsReview,
            "review",
            "nas",
            &nas.source_ref,
            Some("router"),
            None,
            Some(&nas.nas_name),
            Some("Lebih dari satu router cocok; pilih mapping manual."),
        ),
        _ => decision(
            MixradiusImportConflictState::Blocked,
            "blocked",
            "nas",
            &nas.source_ref,
            Some("router"),
            None,
            Some(&nas.nas_name),
            Some("Router MixRadius belum dipetakan; import PPPoE diblokir sampai admin memilih router target."),
        ),
    }
}

pub fn resolve_customer_match(
    customer: &StagedCustomer,
    existing_customers: &[ExistingCustomer],
    external_refs: &[MixradiusExternalRef],
) -> MixradiusMapperDecision {
    if let Some(existing_ref) = external_refs.iter().find(|existing_ref| {
        existing_ref.entity_type == "customer"
            && (existing_ref.source_ref == customer.member_id
                || existing_ref.source_ref == customer.source_ref)
    }) {
        return decision(
            MixradiusImportConflictState::AutoMatched,
            "reuse",
            "customer",
            &customer.member_id,
            Some("customer"),
            Some(&existing_ref.entity_id),
            customer.fullname.as_deref(),
            Some("Customer sudah punya external ref MixRadius, jadi aman di-reuse untuk idempotency."),
        );
    }

    if let Some(existing) = existing_customers.iter().find(|existing| {
        matches_normalized(customer.email.as_deref(), existing.email.as_deref())
            || matches_normalized(customer.phonenumber.as_deref(), existing.phone.as_deref())
            || matches_normalized(customer.fullname.as_deref(), Some(&existing.name))
    }) {
        return decision(
            MixradiusImportConflictState::NeedsReview,
            "review",
            "customer",
            &customer.member_id,
            Some("customer"),
            Some(&existing.id),
            customer.fullname.as_deref(),
            Some("Customer mirip dengan data lokal; admin perlu memilih merge atau create new."),
        );
    }

    decision(
        MixradiusImportConflictState::NeedsReview,
        "create",
        "customer",
        &customer.member_id,
        Some("customer"),
        None,
        customer.fullname.as_deref(),
        Some("Customer belum dikenal; preview akan membuat customer baru bila disetujui."),
    )
}

pub fn resolve_pppoe_action(
    username: &str,
    target_router_id: Option<&str>,
    target_customer_id: Option<&str>,
    existing_accounts: &[ExistingPppoeAccount],
) -> MixradiusPppoeDecision {
    let username_key = normalize_key(username);

    let Some(router_id) = target_router_id else {
        return MixradiusPppoeDecision {
            state: MixradiusImportConflictState::Blocked,
            action: MixradiusPppoeAction::Blocked,
            target_id: None,
            notes: Some("Router target belum dipetakan.".into()),
        };
    };

    if let Some(existing) = existing_accounts
        .iter()
        .find(|account| normalize_key(&account.username) == username_key)
    {
        if existing.router_id != router_id {
            return MixradiusPppoeDecision {
                state: MixradiusImportConflictState::Conflict,
                action: MixradiusPppoeAction::Blocked,
                target_id: Some(existing.id.clone()),
                notes: Some("Username PPP sudah dipakai di router lain; perlu review agar tidak salah migrasi pelanggan.".into()),
            };
        }

        if target_customer_id.is_some_and(|customer_id| customer_id == existing.customer_id) {
            return MixradiusPppoeDecision {
                state: MixradiusImportConflictState::AutoMatched,
                action: MixradiusPppoeAction::Same,
                target_id: Some(existing.id.clone()),
                notes: Some("Akun PPPoE sudah cocok dengan router dan customer target.".into()),
            };
        }

        return MixradiusPppoeDecision {
            state: MixradiusImportConflictState::NeedsReview,
            action: MixradiusPppoeAction::Update,
            target_id: Some(existing.id.clone()),
            notes: Some("Username PPPoE cocok di router target, tetapi customer berbeda; perlu review sebelum update.".into()),
        };
    }

    MixradiusPppoeDecision {
        state: MixradiusImportConflictState::NeedsReview,
        action: MixradiusPppoeAction::New,
        target_id: None,
        notes: Some("Akun PPPoE baru akan dibuat setelah mapping customer/router valid.".into()),
    }
}

pub fn safe_customer_update_patch(
    source: &StagedCustomer,
    existing: &ExistingCustomer,
) -> CustomerUpdatePatch {
    let mut patch = CustomerUpdatePatch::default();

    patch.name = preserve_or_fill(
        "name",
        source.fullname.as_deref(),
        Some(existing.name.as_str()),
        &mut patch.warnings,
    );
    patch.email = preserve_or_fill(
        "email",
        source.email.as_deref(),
        existing.email.as_deref(),
        &mut patch.warnings,
    );
    patch.phone = preserve_or_fill(
        "phone",
        source.phonenumber.as_deref(),
        existing.phone.as_deref(),
        &mut patch.warnings,
    );

    patch
}

fn decision(
    state: MixradiusImportConflictState,
    action: &str,
    source_kind: &str,
    source_ref: &str,
    target_kind: Option<&str>,
    target_id: Option<&str>,
    display_name: Option<&str>,
    notes: Option<&str>,
) -> MixradiusMapperDecision {
    MixradiusMapperDecision {
        state,
        action: action.into(),
        source_kind: source_kind.into(),
        source_ref: source_ref.into(),
        target_kind: target_kind.map(str::to_string),
        target_id: target_id.map(str::to_string),
        display_name: display_name.map(str::to_string),
        notes: notes.map(str::to_string),
    }
}

fn preserve_or_fill(
    field: &str,
    source: Option<&str>,
    existing: Option<&str>,
    warnings: &mut Vec<String>,
) -> Option<String> {
    let source = normalize_optional(source)?;
    match normalize_optional(existing) {
        None => Some(source),
        Some(existing) if normalize_key(&existing) == normalize_key(&source) => None,
        Some(_) => {
            warnings.push(format!(
                "Preserving local edit for customer {field}; MixRadius value differs."
            ));
            None
        }
    }
}

fn matches_normalized(left: Option<&str>, right: Option<&str>) -> bool {
    match (normalize_optional(left), normalize_optional(right)) {
        (Some(left), Some(right)) => normalize_key(&left) == normalize_key(&right),
        _ => false,
    }
}

fn normalize_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn normalize_host(value: &str) -> String {
    value
        .split('/')
        .next()
        .unwrap_or(value)
        .trim()
        .to_ascii_lowercase()
}

fn normalize_key(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}
