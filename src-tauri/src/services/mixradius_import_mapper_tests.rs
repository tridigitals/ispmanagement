#[cfg(test)]
mod mixradius_import_mapper_tests {
    use crate::models::MixradiusImportConflictState;
    use crate::services::mixradius_import_mapper::{
        resolve_customer_match, resolve_package_mapping, resolve_pppoe_action,
        resolve_router_mapping, safe_customer_update_patch, ExistingCustomer, ExistingPackage,
        ExistingPppoeAccount, MixradiusExternalRef, MixradiusImportMapperPolicy,
        MixradiusMapperDecision, MixradiusPppoeAction, StagedCustomer, StagedNas, StagedPlan,
    };
    use chrono::{TimeZone, Utc};
    use serde_json::json;

    #[test]
    fn mixradius_import_mapper_reuses_exact_package_name() {
        let plan = StagedPlan {
            source_ref: "plan-10".into(),
            plan_name: " Paket 10 Mbps ".into(),
            price: Some(150_000.0),
        };
        let packages = vec![ExistingPackage {
            id: "pkg-10".into(),
            name: "paket 10 mbps".into(),
            price_monthly: 150_000.0,
            is_active: true,
        }];

        let decision = resolve_package_mapping(&plan, &packages, &[]);

        assert_eq!(decision.state, MixradiusImportConflictState::AutoMatched);
        assert_eq!(decision.target_kind.as_deref(), Some("package"));
        assert_eq!(decision.target_id.as_deref(), Some("pkg-10"));
        assert_eq!(decision.action, "reuse");
    }

    #[test]
    fn mixradius_import_mapper_blocks_unresolved_router_mapping() {
        let nas = StagedNas {
            source_ref: "nas-5".into(),
            nas_name: "Deres".into(),
            nas_ip_or_cidr: "10.10.10.1".into(),
        };

        let decision = resolve_router_mapping(&nas, &[], &[]);

        assert_eq!(decision.state, MixradiusImportConflictState::Blocked);
        assert_eq!(decision.target_kind.as_deref(), Some("router"));
        assert!(decision
            .notes
            .as_deref()
            .unwrap_or_default()
            .contains("Router MixRadius belum dipetakan"));
    }

    #[test]
    fn mixradius_import_mapper_flags_ppp_username_conflict_across_routers() {
        let existing = vec![ExistingPppoeAccount {
            id: "ppp-existing".into(),
            router_id: "router-a".into(),
            username: "pelanggan001".into(),
            customer_id: "customer-a".into(),
            disabled: false,
        }];

        let decision = resolve_pppoe_action(
            "pelanggan001",
            Some("router-b"),
            Some("customer-b"),
            &existing,
        );

        assert_eq!(decision.state, MixradiusImportConflictState::Conflict);
        assert_eq!(decision.action, MixradiusPppoeAction::Blocked);
        assert_eq!(decision.target_id.as_deref(), Some("ppp-existing"));
    }

    #[test]
    fn mixradius_import_mapper_auto_matches_customer_external_ref() {
        let customer = StagedCustomer {
            source_ref: "customer-row-77".into(),
            member_id: "MBR-77".into(),
            username: Some("mbr77".into()),
            fullname: Some("Budi Local".into()),
            email: Some("budi@example.test".into()),
            phonenumber: Some("081234".into()),
            trx_status: Some("PAID".into()),
            expired_on: Some(Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap()),
        };
        let refs = vec![MixradiusExternalRef {
            entity_type: "customer".into(),
            entity_id: "customer-local-77".into(),
            source_ref: "MBR-77".into(),
        }];

        let decision = resolve_customer_match(&customer, &[], &refs);

        assert_eq!(decision.state, MixradiusImportConflictState::AutoMatched);
        assert_eq!(decision.target_kind.as_deref(), Some("customer"));
        assert_eq!(decision.target_id.as_deref(), Some("customer-local-77"));
    }

    #[test]
    fn mixradius_import_mapper_normalizes_mixradius_billing_lifecycle() {
        let now = Utc.with_ymd_and_hms(2026, 4, 11, 12, 0, 0).unwrap();

        let paid = MixradiusImportMapperPolicy::normalize_subscription_lifecycle(
            Some("PAID"),
            Some(Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap()),
            now,
        );
        assert_eq!(paid.status, "active");
        assert!(paid.warnings.is_empty());

        let unpaid_expired = MixradiusImportMapperPolicy::normalize_subscription_lifecycle(
            Some("UNPAID"),
            Some(Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap()),
            now,
        );
        assert_eq!(unpaid_expired.status, "suspended");
        assert!(unpaid_expired.warnings.is_empty());

        let unpaid_current = MixradiusImportMapperPolicy::normalize_subscription_lifecycle(
            Some("UNPAID"),
            Some(Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap()),
            now,
        );
        assert_eq!(unpaid_current.status, "active");
        assert!(unpaid_current
            .warnings
            .iter()
            .any(|warning: &String| warning.contains("belum lunas")));

        let pending = MixradiusImportMapperPolicy::normalize_subscription_lifecycle(
            Some("PENDING"),
            Some(Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap()),
            now,
        );
        assert_eq!(pending.status, "pending_installation");
        assert!(pending.requires_review);
    }

    #[test]
    fn mixradius_import_mapper_conflict_states_serialize_as_preview_contract() {
        let states = vec![
            MixradiusImportConflictState::AutoMatched,
            MixradiusImportConflictState::NeedsReview,
            MixradiusImportConflictState::Conflict,
            MixradiusImportConflictState::Blocked,
            MixradiusImportConflictState::Skipped,
        ];

        assert_eq!(
            serde_json::to_value(states).expect("states should serialize"),
            json!([
                "auto_matched",
                "needs_review",
                "conflict",
                "blocked",
                "skipped"
            ])
        );
    }

    #[test]
    fn mixradius_import_mapper_safe_mode_preserves_local_customer_profile_edits() {
        let source = StagedCustomer {
            source_ref: "row-88".into(),
            member_id: "MBR-88".into(),
            username: Some("mbr88".into()),
            fullname: Some("Nama Dari MixRadius".into()),
            email: Some("mixradius@example.test".into()),
            phonenumber: Some("089999".into()),
            trx_status: Some("PAID".into()),
            expired_on: Some(Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap()),
        };
        let existing = ExistingCustomer {
            id: "customer-local-88".into(),
            name: "Nama Sudah Diedit Lokal".into(),
            email: Some("lokal@example.test".into()),
            phone: Some("087777".into()),
            is_active: true,
        };

        let patch = safe_customer_update_patch(&source, &existing);

        assert_eq!(patch.name, None);
        assert_eq!(patch.email, None);
        assert_eq!(patch.phone, None);
        assert!(patch
            .warnings
            .iter()
            .any(|warning: &String| warning.contains("local edit")));
    }

    #[test]
    fn mixradius_mapper_decision_serializes_for_api_preview_rows() {
        let decision = MixradiusMapperDecision {
            state: MixradiusImportConflictState::NeedsReview,
            action: "create".into(),
            source_kind: "customer".into(),
            source_ref: "MBR-1".into(),
            target_kind: Some("customer".into()),
            target_id: None,
            display_name: Some("Budi".into()),
            notes: Some("Perlu review".into()),
        };

        assert_eq!(
            serde_json::to_value(decision).expect("decision should serialize"),
            json!({
                "state": "needs_review",
                "action": "create",
                "sourceKind": "customer",
                "sourceRef": "MBR-1",
                "targetKind": "customer",
                "targetId": null,
                "displayName": "Budi",
                "notes": "Perlu review"
            })
        );
    }
}
