use crate::db::DbPool;
use crate::models::{
    MixradiusImportExecutionMode, MixradiusImportMappingOverride,
    MixradiusImportPppoeProvisioningTarget, PppoeAccount, PppoeAccountSource,
};
use crate::security::secret::decrypt_secret_opt_for;
use crate::services::managed_radius_service::ManagedRadiusService;
use crate::services::mixradius_import_mapper::{
    safe_customer_update_patch, ExistingCustomer, MixradiusImportMapperPolicy, StagedCustomer,
};
use crate::services::pppoe_service::{encrypt_pppoe_password_for_storage, PURPOSE_PPPOE};
use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MixradiusPackageExecutionSummary {
    pub total_rows: i64,
    pub imported_rows: i64,
    pub updated_rows: i64,
    pub skipped_rows: i64,
    pub conflict_rows: i64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MixradiusCustomerExecutionSummary {
    pub total_rows: i64,
    pub imported_rows: i64,
    pub updated_rows: i64,
    pub skipped_rows: i64,
    pub conflict_rows: i64,
    pub location_imported_rows: i64,
    pub location_updated_rows: i64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MixradiusSubscriptionExecutionSummary {
    pub total_rows: i64,
    pub imported_rows: i64,
    pub updated_rows: i64,
    pub skipped_rows: i64,
    pub conflict_rows: i64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MixradiusPppoeExecutionSummary {
    pub total_rows: i64,
    pub imported_rows: i64,
    pub updated_rows: i64,
    pub skipped_rows: i64,
    pub conflict_rows: i64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, FromRow)]
struct StagedPlanRow {
    source_ref: String,
    plan_name: String,
    bandwidth_name: Option<String>,
    price: Option<f64>,
    validity: Option<String>,
    shared_users: Option<i32>,
}

#[derive(Debug, Clone, FromRow)]
struct ExistingPackageRow {
    id: String,
    name: String,
    price_monthly: f64,
}

#[derive(Debug, Clone, FromRow)]
struct RouterProfileCandidateRow {
    name: String,
    rate_limit: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
struct StagedCustomerRow {
    source_ref: String,
    member_id: String,
    username: Option<String>,
    password: Option<String>,
    fullname: Option<String>,
    email: Option<String>,
    phonenumber: Option<String>,
    address: Option<String>,
    plan_name: Option<String>,
    price: Option<f64>,
    renewed_on: Option<chrono::DateTime<Utc>>,
    trx_status: Option<String>,
    expired_on: Option<chrono::DateTime<Utc>>,
    source_json: Value,
}

#[derive(Debug, Clone, FromRow)]
struct StagedCustomerLocationRow {
    source_ref: String,
    member_id: String,
    latitude: Option<f64>,
    longitude: Option<f64>,
}

#[derive(Debug, Clone, FromRow)]
struct ExistingCustomerRow {
    id: String,
    name: String,
    email: Option<String>,
    phone: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
struct ExistingCustomerLocationRow {
    id: String,
    label: String,
    notes: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
struct ExistingSubscriptionRow {
    #[sqlx(rename = "id")]
    _id: String,
    #[sqlx(rename = "location_id")]
    _location_id: String,
    #[sqlx(rename = "status")]
    _status: String,
}

#[derive(Debug, Clone, FromRow)]
struct ExistingPppoeAccountRow {
    id: String,
    router_id: String,
}

#[derive(Clone)]
pub struct MixradiusImportExecutor {
    pool: DbPool,
}

impl MixradiusImportExecutor {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn execute_package_imports(
        &self,
        tenant_id: &str,
        batch_id: &str,
        mapping_overrides: &[MixradiusImportMappingOverride],
    ) -> Result<MixradiusPackageExecutionSummary> {
        self.execute_package_imports_with_mode(
            tenant_id,
            batch_id,
            mapping_overrides,
            MixradiusImportExecutionMode::SafeImport,
        )
        .await
    }

    pub async fn execute_package_imports_with_mode(
        &self,
        tenant_id: &str,
        batch_id: &str,
        mapping_overrides: &[MixradiusImportMappingOverride],
        execution_mode: MixradiusImportExecutionMode,
    ) -> Result<MixradiusPackageExecutionSummary> {
        let staged_plans = sqlx::query_as::<_, StagedPlanRow>(
            r#"
            SELECT
                source_ref,
                plan_name,
                bandwidth_name,
                price::float8 AS price,
                validity,
                shared_users
            FROM public.mixradius_staging_plans
            WHERE tenant_id = $1 AND import_batch_id = $2
              AND COALESCE(source_json->'values'->>8, 'PPP') = 'PPP'
            ORDER BY created_at ASC
            "#,
        )
        .bind(tenant_id)
        .bind(batch_id)
        .fetch_all(&self.pool)
        .await
        .context("failed to load MixRadius staged plans for execution")?;

        let mut summary = MixradiusPackageExecutionSummary {
            total_rows: staged_plans.len() as i64,
            ..Default::default()
        };

        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to open MixRadius package execution transaction")?;
        let target_router_id =
            resolve_mixradius_router_id(&mut tx, tenant_id, batch_id, mapping_overrides).await?;

        for plan in staged_plans {
            if let Some(target_package_id) =
                find_override_target(mapping_overrides, "plan", &plan.source_ref)
            {
                let existing_override: Option<String> = sqlx::query_scalar(
                    r#"
                    SELECT id
                    FROM public.isp_packages
                    WHERE tenant_id = $1 AND id = $2
                    "#,
                )
                .bind(tenant_id)
                .bind(target_package_id)
                .fetch_optional(&mut *tx)
                .await
                .context("failed to validate package override target")?;

                if let Some(entity_id) = existing_override {
                    upsert_external_ref(
                        &mut tx,
                        tenant_id,
                        batch_id,
                        "package",
                        &entity_id,
                        &plan.source_ref,
                    )
                    .await?;
                    upsert_package_router_mapping_for_mixradius_plan(
                        &mut tx,
                        tenant_id,
                        target_router_id.as_deref(),
                        &entity_id,
                        &plan,
                        &mut summary,
                    )
                    .await?;
                    summary.updated_rows += 1;
                    continue;
                }
            }

            if let Some(existing_ref_package_id) = sqlx::query_scalar::<_, String>(
                r#"
                SELECT entity_id
                FROM public.mixradius_import_external_refs
                WHERE tenant_id = $1
                  AND source_system = 'mixradius'
                  AND entity_type = 'package'
                  AND source_ref = $2
                "#,
            )
            .bind(tenant_id)
            .bind(&plan.source_ref)
            .fetch_optional(&mut *tx)
            .await
            .context("failed to load MixRadius external package ref")?
            {
                let package_still_exists: Option<String> = sqlx::query_scalar(
                    "SELECT id FROM public.isp_packages WHERE tenant_id = $1 AND id = $2",
                )
                .bind(tenant_id)
                .bind(&existing_ref_package_id)
                .fetch_optional(&mut *tx)
                .await
                .context("failed to verify external package ref target")?;

                if let Some(entity_id) = package_still_exists {
                    upsert_external_ref(
                        &mut tx,
                        tenant_id,
                        batch_id,
                        "package",
                        &entity_id,
                        &plan.source_ref,
                    )
                    .await?;
                    upsert_package_router_mapping_for_mixradius_plan(
                        &mut tx,
                        tenant_id,
                        target_router_id.as_deref(),
                        &entity_id,
                        &plan,
                        &mut summary,
                    )
                    .await?;
                    summary.updated_rows += 1;
                    continue;
                }
            }

            let normalized_name = normalize_name(&plan.plan_name);
            let same_name_package = sqlx::query_as::<_, ExistingPackageRow>(
                r#"
                SELECT
                    id,
                    name,
                    price_monthly::float8 AS price_monthly
                FROM public.isp_packages
                WHERE tenant_id = $1
                ORDER BY created_at ASC
                "#,
            )
            .bind(tenant_id)
            .fetch_all(&mut *tx)
            .await
            .context("failed to load existing packages during MixRadius execution")?
            .into_iter()
            .find(|package| normalize_name(&package.name) == normalized_name);

            if let Some(package) = same_name_package {
                if same_price(package.price_monthly, plan.price) {
                    upsert_external_ref(
                        &mut tx,
                        tenant_id,
                        batch_id,
                        "package",
                        &package.id,
                        &plan.source_ref,
                    )
                    .await?;
                    upsert_package_router_mapping_for_mixradius_plan(
                        &mut tx,
                        tenant_id,
                        target_router_id.as_deref(),
                        &package.id,
                        &plan,
                        &mut summary,
                    )
                    .await?;
                    summary.updated_rows += 1;
                    continue;
                }

                if execution_mode == MixradiusImportExecutionMode::ForceSync {
                    let now = Utc::now();
                    sqlx::query(
                        r#"
                        UPDATE public.isp_packages
                        SET description = $1,
                            features = $2,
                            price_monthly = $3,
                            updated_at = $4
                        WHERE tenant_id = $5 AND id = $6
                        "#,
                    )
                    .bind(build_package_description(&plan))
                    .bind(build_package_features(&plan))
                    .bind(plan.price.unwrap_or(package.price_monthly))
                    .bind(now)
                    .bind(tenant_id)
                    .bind(&package.id)
                    .execute(&mut *tx)
                    .await
                    .context("failed to force-sync MixRadius package price mismatch")?;

                    upsert_external_ref(
                        &mut tx,
                        tenant_id,
                        batch_id,
                        "package",
                        &package.id,
                        &plan.source_ref,
                    )
                    .await?;
                    upsert_package_router_mapping_for_mixradius_plan(
                        &mut tx,
                        tenant_id,
                        target_router_id.as_deref(),
                        &package.id,
                        &plan,
                        &mut summary,
                    )
                    .await?;
                    summary.updated_rows += 1;
                    summary.warnings.push(format!(
                        "Force sync menimpa harga package `{}` mengikuti data MixRadius.",
                        plan.plan_name.trim()
                    ));
                    continue;
                } else {
                    insert_conflict(
                        &mut tx,
                        tenant_id,
                        batch_id,
                        &plan.source_ref,
                        "package_price_mismatch",
                        &format!(
                            "Package `{}` sudah ada, tetapi harga MixRadius berbeda dan perlu review manual.",
                            plan.plan_name.trim()
                        ),
                        json!({
                            "existingPackageId": package.id,
                            "existingPriceMonthly": package.price_monthly,
                            "sourcePriceMonthly": plan.price,
                        }),
                    )
                    .await?;
                    summary.conflict_rows += 1;
                    continue;
                }
            }

            let package_id = Uuid::new_v4().to_string();
            let package_name = plan.plan_name.trim().to_string();
            let description = build_package_description(&plan);
            let features = build_package_features(&plan);

            sqlx::query(
                r#"
                INSERT INTO public.isp_packages (
                    id,
                    tenant_id,
                    service_type,
                    name,
                    description,
                    features,
                    is_active,
                    price_monthly,
                    price_yearly,
                    created_at,
                    updated_at
                )
                VALUES ($1, $2, 'internet_pppoe', $3, $4, $5, true, $6, 0, $7, $7)
                "#,
            )
            .bind(&package_id)
            .bind(tenant_id)
            .bind(&package_name)
            .bind(description)
            .bind(features)
            .bind(plan.price.unwrap_or(0.0))
            .bind(Utc::now())
            .execute(&mut *tx)
            .await
            .with_context(|| {
                format!("failed to create ISP package for MixRadius plan `{package_name}`")
            })?;

            upsert_external_ref(
                &mut tx,
                tenant_id,
                batch_id,
                "package",
                &package_id,
                &plan.source_ref,
            )
            .await?;
            upsert_package_router_mapping_for_mixradius_plan(
                &mut tx,
                tenant_id,
                target_router_id.as_deref(),
                &package_id,
                &plan,
                &mut summary,
            )
            .await?;
            summary.imported_rows += 1;
        }

        tx.commit()
            .await
            .context("failed to commit MixRadius package execution transaction")?;

        Ok(summary)
    }

    pub async fn execute_customer_imports(
        &self,
        tenant_id: &str,
        batch_id: &str,
    ) -> Result<MixradiusCustomerExecutionSummary> {
        let staged_customers = sqlx::query_as::<_, StagedCustomerRow>(
            r#"
            SELECT
                source_ref,
                member_id,
                username,
                password,
                fullname,
                email,
                phonenumber,
                address,
                plan_name,
                price::float8 AS price,
                renewed_on,
                trx_status,
                expired_on,
                source_json
            FROM public.mixradius_staging_customers
            WHERE tenant_id = $1 AND import_batch_id = $2
              AND COALESCE(source_json->'values'->>3, 'PPP') = 'PPP'
            ORDER BY created_at ASC
            "#,
        )
        .bind(tenant_id)
        .bind(batch_id)
        .fetch_all(&self.pool)
        .await
        .context("failed to load MixRadius staged customers for execution")?;

        let staged_locations = sqlx::query_as::<_, StagedCustomerLocationRow>(
            r#"
            SELECT source_ref, member_id, latitude::float8 AS latitude, longitude::float8 AS longitude
            FROM public.mixradius_staging_customer_locations
            WHERE tenant_id = $1 AND import_batch_id = $2
            ORDER BY created_at ASC
            "#,
        )
        .bind(tenant_id)
        .bind(batch_id)
        .fetch_all(&self.pool)
        .await
        .context("failed to load MixRadius staged customer locations for execution")?;

        let mut summary = MixradiusCustomerExecutionSummary {
            total_rows: staged_customers.len() as i64,
            ..Default::default()
        };

        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to open MixRadius customer execution transaction")?;

        for staged_customer in staged_customers {
            let customer_source_ref = staged_customer.member_id.clone();
            let existing_customer_id =
                find_external_ref_entity_id(&mut tx, tenant_id, "customer", &customer_source_ref)
                    .await?;

            let (customer_id, customer_created) =
                if let Some(existing_customer_id) = existing_customer_id {
                    let existing_customer = sqlx::query_as::<_, ExistingCustomerRow>(
                        r#"
                    SELECT id, name, email, phone, notes
                    FROM public.customers
                    WHERE tenant_id = $1 AND id = $2
                    "#,
                    )
                    .bind(tenant_id)
                    .bind(&existing_customer_id)
                    .fetch_optional(&mut *tx)
                    .await
                    .context("failed to load existing customer from MixRadius external ref")?;

                    if let Some(existing_customer) = existing_customer {
                        let patch = safe_customer_update_patch(
                            &StagedCustomer {
                                source_ref: staged_customer.source_ref.clone(),
                                member_id: staged_customer.member_id.clone(),
                                username: staged_customer.username.clone(),
                                fullname: staged_customer.fullname.clone(),
                                email: staged_customer.email.clone(),
                                phonenumber: staged_customer.phonenumber.clone(),
                                trx_status: staged_customer.trx_status.clone(),
                                expired_on: staged_customer.expired_on,
                            },
                            &ExistingCustomer {
                                id: existing_customer.id.clone(),
                                name: existing_customer.name.clone(),
                                email: existing_customer.email.clone(),
                                phone: existing_customer.phone.clone(),
                                is_active: true,
                            },
                        );
                        summary.warnings.extend(patch.warnings);

                        sqlx::query(
                            r#"
                        UPDATE public.customers
                        SET name = COALESCE($1, name),
                            email = COALESCE($2, email),
                            phone = COALESCE($3, phone),
                            updated_at = $4
                        WHERE tenant_id = $5 AND id = $6
                        "#,
                        )
                        .bind(patch.name)
                        .bind(patch.email)
                        .bind(patch.phone)
                        .bind(Utc::now())
                        .bind(tenant_id)
                        .bind(&existing_customer.id)
                        .execute(&mut *tx)
                        .await
                        .context("failed to update existing imported customer safely")?;

                        if existing_customer.notes.is_none() {
                            let imported_notes = build_customer_notes(&staged_customer);
                            sqlx::query(
                                r#"
                            UPDATE public.customers
                            SET notes = $1, updated_at = $2
                            WHERE tenant_id = $3 AND id = $4
                              AND notes IS NULL
                            "#,
                            )
                            .bind(imported_notes)
                            .bind(Utc::now())
                            .bind(tenant_id)
                            .bind(&existing_customer.id)
                            .execute(&mut *tx)
                            .await
                            .context("failed to backfill imported customer notes")?;
                        }

                        upsert_external_ref(
                            &mut tx,
                            tenant_id,
                            batch_id,
                            "customer",
                            &existing_customer.id,
                            &customer_source_ref,
                        )
                        .await?;
                        summary.updated_rows += 1;
                        (existing_customer.id, false)
                    } else {
                        let customer_id = insert_customer(
                            &mut tx,
                            tenant_id,
                            &staged_customer,
                            &customer_source_ref,
                            batch_id,
                        )
                        .await?;
                        (customer_id, true)
                    }
                } else {
                    let customer_id = insert_customer(
                        &mut tx,
                        tenant_id,
                        &staged_customer,
                        &customer_source_ref,
                        batch_id,
                    )
                    .await?;
                    (customer_id, true)
                };
            if customer_created {
                summary.imported_rows += 1;
            }

            if summary.updated_rows + summary.imported_rows > 0 {
                if find_external_ref_entity_id(&mut tx, tenant_id, "customer", &customer_source_ref)
                    .await?
                    .as_deref()
                    == Some(customer_id.as_str())
                {
                    if !sqlx::query_scalar::<_, bool>(
                        "SELECT EXISTS(SELECT 1 FROM public.customers WHERE tenant_id = $1 AND id = $2)",
                    )
                    .bind(tenant_id)
                    .bind(&customer_id)
                    .fetch_one(&mut *tx)
                    .await
                    .context("failed to verify imported customer")?
                    {
                        continue;
                    }
                }
            }

            let customer_location = staged_locations
                .iter()
                .find(|location| location.member_id == staged_customer.member_id);
            let location_source_ref = customer_location
                .map(|location| location.source_ref.clone())
                .unwrap_or_else(|| {
                    format!("customer:{}:default-location", staged_customer.member_id)
                });

            let existing_location_id =
                find_external_ref_entity_id(&mut tx, tenant_id, "location", &location_source_ref)
                    .await?;
            let staged_address = normalize_optional(staged_customer.address.as_deref());
            let staged_coords = customer_location
                .map(|location| (location.latitude, location.longitude))
                .unwrap_or((None, None));

            if let Some(existing_location_id) = existing_location_id {
                let existing_location = sqlx::query_as::<_, ExistingCustomerLocationRow>(
                    r#"
                    SELECT id, label, notes
                    FROM public.customer_locations
                    WHERE tenant_id = $1 AND id = $2
                    "#,
                )
                .bind(tenant_id)
                .bind(&existing_location_id)
                .fetch_optional(&mut *tx)
                .await
                .context("failed to load existing imported location")?;

                if let Some(existing_location) = existing_location {
                    sqlx::query(
                        r#"
                        UPDATE public.customer_locations
                        SET customer_id = $1,
                            label = $2,
                            address_line1 = COALESCE($3, address_line1),
                            latitude = COALESCE($4, latitude),
                            longitude = COALESCE($5, longitude),
                            updated_at = $6
                        WHERE tenant_id = $7 AND id = $8
                        "#,
                    )
                    .bind(&customer_id)
                    .bind(existing_location.label)
                    .bind(staged_address.clone())
                    .bind(staged_coords.0)
                    .bind(staged_coords.1)
                    .bind(Utc::now())
                    .bind(tenant_id)
                    .bind(&existing_location.id)
                    .execute(&mut *tx)
                    .await
                    .context("failed to update imported customer location")?;

                    if existing_location.notes.is_none() {
                        let notes = build_location_notes(&staged_customer);
                        sqlx::query(
                            r#"
                            UPDATE public.customer_locations
                            SET notes = $1, updated_at = $2
                            WHERE tenant_id = $3 AND id = $4
                              AND notes IS NULL
                            "#,
                        )
                        .bind(notes)
                        .bind(Utc::now())
                        .bind(tenant_id)
                        .bind(&existing_location.id)
                        .execute(&mut *tx)
                        .await
                        .context("failed to backfill imported location notes")?;
                    }

                    upsert_external_ref(
                        &mut tx,
                        tenant_id,
                        batch_id,
                        "location",
                        &existing_location.id,
                        &location_source_ref,
                    )
                    .await?;
                    summary.location_updated_rows += 1;
                } else {
                    create_location_for_customer(
                        &mut tx,
                        tenant_id,
                        batch_id,
                        &customer_id,
                        &location_source_ref,
                        staged_address.clone(),
                        staged_coords.0,
                        staged_coords.1,
                        build_location_notes(&staged_customer),
                    )
                    .await?;
                    summary.location_imported_rows += 1;
                }
            } else {
                create_location_for_customer(
                    &mut tx,
                    tenant_id,
                    batch_id,
                    &customer_id,
                    &location_source_ref,
                    staged_address.clone(),
                    staged_coords.0,
                    staged_coords.1,
                    build_location_notes(&staged_customer),
                )
                .await?;
                summary.location_imported_rows += 1;
            }
        }

        tx.commit()
            .await
            .context("failed to commit MixRadius customer execution transaction")?;

        Ok(summary)
    }

    pub async fn execute_subscription_imports(
        &self,
        tenant_id: &str,
        batch_id: &str,
    ) -> Result<MixradiusSubscriptionExecutionSummary> {
        let staged_customers = sqlx::query_as::<_, StagedCustomerRow>(
            r#"
            SELECT
                source_ref,
                member_id,
                username,
                password,
                fullname,
                email,
                phonenumber,
                address,
                plan_name,
                price::float8 AS price,
                renewed_on,
                trx_status,
                expired_on,
                source_json
            FROM public.mixradius_staging_customers
            WHERE tenant_id = $1 AND import_batch_id = $2
              AND COALESCE(source_json->'values'->>3, 'PPP') = 'PPP'
            ORDER BY created_at ASC
            "#,
        )
        .bind(tenant_id)
        .bind(batch_id)
        .fetch_all(&self.pool)
        .await
        .context("failed to load MixRadius staged customers for subscription execution")?;

        let mut summary = MixradiusSubscriptionExecutionSummary {
            total_rows: staged_customers.len() as i64,
            ..Default::default()
        };

        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to open MixRadius subscription execution transaction")?;

        for staged_customer in staged_customers {
            let Some(customer_id) = find_external_ref_entity_id(
                &mut tx,
                tenant_id,
                "customer",
                &staged_customer.member_id,
            )
            .await?
            else {
                continue;
            };
            let Some(location_id) = find_imported_location_id_for_member(
                &mut tx,
                tenant_id,
                batch_id,
                &staged_customer.member_id,
            )
            .await?
            else {
                continue;
            };
            let Some(package_id) =
                find_package_id_for_customer(&mut tx, tenant_id, &staged_customer).await?
            else {
                insert_generic_conflict(
                    &mut tx,
                    tenant_id,
                    batch_id,
                    "tbl_customers",
                    &staged_customer.member_id,
                    "subscription_package_missing",
                    "Package untuk subscription MixRadius belum ditemukan; execute subscription dilewati.",
                )
                .await?;
                summary.conflict_rows += 1;
                continue;
            };

            let lifecycle = MixradiusImportMapperPolicy::normalize_subscription_lifecycle(
                staged_customer.trx_status.as_deref(),
                staged_customer.expired_on,
                Utc::now(),
            );
            let mut notes = vec![format!(
                "Imported from MixRadius member {}",
                staged_customer.member_id
            )];
            notes.extend(lifecycle.warnings.clone());
            summary.warnings.extend(lifecycle.warnings.clone());

            let existing_subscription_id = find_external_ref_entity_id(
                &mut tx,
                tenant_id,
                "subscription",
                &staged_customer.member_id,
            )
            .await?;

            if let Some(existing_subscription_id) = existing_subscription_id {
                sqlx::query(
                    r#"
                    UPDATE public.customer_subscriptions
                    SET customer_id = $1,
                        location_id = $2,
                        package_id = $3,
                        billing_cycle = $4,
                        price = $5,
                        currency_code = 'IDR',
                        status = $6,
                        starts_at = $7,
                        ends_at = $8,
                        notes = $9,
                        updated_at = $10
                    WHERE tenant_id = $11 AND id = $12
                    "#,
                )
                .bind(&customer_id)
                .bind(&location_id)
                .bind(&package_id)
                .bind(resolve_billing_cycle(&staged_customer))
                .bind(staged_customer.price.unwrap_or(0.0))
                .bind(lifecycle.status)
                .bind(staged_customer.renewed_on)
                .bind(staged_customer.expired_on)
                .bind(Some(notes.join(" | ")))
                .bind(Utc::now())
                .bind(tenant_id)
                .bind(&existing_subscription_id)
                .execute(&mut *tx)
                .await
                .context("failed to update imported MixRadius subscription")?;
                upsert_external_ref(
                    &mut tx,
                    tenant_id,
                    batch_id,
                    "subscription",
                    &existing_subscription_id,
                    &staged_customer.member_id,
                )
                .await?;
                summary.updated_rows += 1;
                continue;
            }

            let location_active_subscription = sqlx::query_as::<_, ExistingSubscriptionRow>(
                r#"
                SELECT id, location_id, status
                FROM public.customer_subscriptions
                WHERE tenant_id = $1
                  AND location_id = $2
                  AND LOWER(status) IN ('active', 'grace_active', 'pending_installation', 'installation_done_awaiting_payment')
                ORDER BY created_at DESC
                LIMIT 1
                "#,
            )
            .bind(tenant_id)
            .bind(&location_id)
            .fetch_optional(&mut *tx)
            .await
            .context("failed to check existing active subscription on location")?;

            if location_active_subscription.is_some() {
                insert_generic_conflict(
                    &mut tx,
                    tenant_id,
                    batch_id,
                    "tbl_customers",
                    &staged_customer.member_id,
                    "subscription_location_conflict",
                    "Lokasi sudah punya subscription aktif lokal; perlu review manual sebelum import subscription MixRadius.",
                )
                .await?;
                summary.conflict_rows += 1;
                continue;
            }

            let subscription_id = Uuid::new_v4().to_string();
            sqlx::query(
                r#"
                INSERT INTO public.customer_subscriptions (
                    id,
                    tenant_id,
                    customer_id,
                    location_id,
                    package_id,
                    router_id,
                    billing_cycle,
                    price,
                    currency_code,
                    status,
                    starts_at,
                    ends_at,
                    grace_started_at,
                    grace_until,
                    notes,
                    created_at,
                    updated_at
                )
                VALUES (
                    $1, $2, $3, $4, $5, NULL, $6, $7, 'IDR', $8, $9, $10, NULL, NULL, $11, $12, $12
                )
                "#,
            )
            .bind(&subscription_id)
            .bind(tenant_id)
            .bind(&customer_id)
            .bind(&location_id)
            .bind(&package_id)
            .bind(resolve_billing_cycle(&staged_customer))
            .bind(staged_customer.price.unwrap_or(0.0))
            .bind(lifecycle.status)
            .bind(staged_customer.renewed_on)
            .bind(staged_customer.expired_on)
            .bind(Some(notes.join(" | ")))
            .bind(Utc::now())
            .execute(&mut *tx)
            .await
            .context("failed to create imported MixRadius subscription")?;

            upsert_external_ref(
                &mut tx,
                tenant_id,
                batch_id,
                "subscription",
                &subscription_id,
                &staged_customer.member_id,
            )
            .await?;
            summary.imported_rows += 1;
        }

        tx.commit()
            .await
            .context("failed to commit MixRadius subscription execution transaction")?;

        Ok(summary)
    }

    pub async fn execute_pppoe_imports(
        &self,
        tenant_id: &str,
        batch_id: &str,
        mapping_overrides: &[MixradiusImportMappingOverride],
    ) -> Result<MixradiusPppoeExecutionSummary> {
        self.execute_pppoe_imports_with_target(
            tenant_id,
            batch_id,
            mapping_overrides,
            MixradiusImportPppoeProvisioningTarget::Router,
        )
        .await
    }

    pub async fn execute_pppoe_imports_with_target(
        &self,
        tenant_id: &str,
        batch_id: &str,
        mapping_overrides: &[MixradiusImportMappingOverride],
        provisioning_target: MixradiusImportPppoeProvisioningTarget,
    ) -> Result<MixradiusPppoeExecutionSummary> {
        let staged_customers = sqlx::query_as::<_, StagedCustomerRow>(
            r#"
            SELECT
                source_ref,
                member_id,
                username,
                password,
                fullname,
                email,
                phonenumber,
                address,
                plan_name,
                price::float8 AS price,
                renewed_on,
                trx_status,
                expired_on,
                source_json
            FROM public.mixradius_staging_customers
            WHERE tenant_id = $1 AND import_batch_id = $2
              AND COALESCE(source_json->'values'->>3, 'PPP') = 'PPP'
            ORDER BY created_at ASC
            "#,
        )
        .bind(tenant_id)
        .bind(batch_id)
        .fetch_all(&self.pool)
        .await
        .context("failed to load MixRadius staged customers for PPPoE execution")?;

        let mut summary = MixradiusPppoeExecutionSummary {
            total_rows: staged_customers.len() as i64,
            ..Default::default()
        };

        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to open MixRadius PPPoE execution transaction")?;

        let account_source = match provisioning_target {
            MixradiusImportPppoeProvisioningTarget::Router => PppoeAccountSource::Router,
            MixradiusImportPppoeProvisioningTarget::ManagedRadius => {
                PppoeAccountSource::ManagedRadius
            }
        };
        let mut pending_managed_radius_sync_ids = Vec::new();

        let Some(router_id) =
            resolve_mixradius_router_id(&mut tx, tenant_id, batch_id, mapping_overrides).await?
        else {
            summary.skipped_rows = summary.total_rows;
            summary.warnings.push(
                "Router target untuk import PPPoE belum dipilih; semua akun PPPoE MixRadius dilewati."
                    .to_string(),
            );
            tx.commit()
                .await
                .context("failed to finalize skipped MixRadius PPPoE transaction")?;
            return Ok(summary);
        };

        for staged_customer in staged_customers {
            let Some(username) = normalize_optional(staged_customer.username.as_deref()) else {
                summary.skipped_rows += 1;
                continue;
            };
            let Some(password) = normalize_optional(staged_customer.password.as_deref()) else {
                summary.skipped_rows += 1;
                summary.warnings.push(format!(
                    "Password PPPoE untuk member {} kosong; akun dilewati.",
                    staged_customer.member_id
                ));
                continue;
            };

            let Some(customer_id) = find_external_ref_entity_id(
                &mut tx,
                tenant_id,
                "customer",
                &staged_customer.member_id,
            )
            .await?
            else {
                summary.skipped_rows += 1;
                continue;
            };
            let Some(location_id) = find_imported_location_id_for_member(
                &mut tx,
                tenant_id,
                batch_id,
                &staged_customer.member_id,
            )
            .await?
            else {
                summary.skipped_rows += 1;
                continue;
            };

            let remote_address = extract_framed_ip_address(&staged_customer.source_json);
            let package_id =
                find_latest_location_package_id(&mut tx, tenant_id, &location_id).await?;
            let password_enc = encrypt_pppoe_password_for_storage(&password)
                .context("failed to encrypt MixRadius PPPoE password")?;
            let notes = format!(
                "Imported from MixRadius member {}",
                staged_customer.member_id
            );
            let existing_external_ref_id = find_external_ref_entity_id(
                &mut tx,
                tenant_id,
                "pppoe_account",
                &staged_customer.member_id,
            )
            .await?;

            let existing_username_row = sqlx::query_as::<_, ExistingPppoeAccountRow>(
                r#"
                SELECT id, router_id
                FROM public.pppoe_accounts
                WHERE tenant_id = $1 AND username = $2
                ORDER BY created_at ASC
                LIMIT 1
                "#,
            )
            .bind(tenant_id)
            .bind(&username)
            .fetch_optional(&mut *tx)
            .await
            .context("failed to resolve existing PPPoE account by username")?;

            let existing_account_id = existing_external_ref_id
                .clone()
                .or_else(|| existing_username_row.as_ref().map(|row| row.id.clone()));

            let mismatched_router = existing_username_row
                .as_ref()
                .filter(|row| row.router_id != router_id);
            if existing_external_ref_id.is_none() && mismatched_router.is_some() {
                insert_generic_conflict(
                    &mut tx,
                    tenant_id,
                    batch_id,
                    "tbl_customers",
                    &staged_customer.member_id,
                    "pppoe_router_mismatch",
                    "Username PPPoE sudah ada pada router lain; perlu review manual sebelum import MixRadius.",
                )
                .await?;
                summary.conflict_rows += 1;
                continue;
            }

            if let Some(existing_account_id) = existing_account_id {
                let target_router_id: Option<String> = sqlx::query_scalar(
                    "SELECT router_id FROM public.pppoe_accounts WHERE tenant_id = $1 AND id = $2",
                )
                .bind(tenant_id)
                .bind(&existing_account_id)
                .fetch_optional(&mut *tx)
                .await
                .context("failed to load target PPPoE account router")?;

                if let Some(target_router_id) = target_router_id {
                    if target_router_id != router_id {
                        insert_generic_conflict(
                            &mut tx,
                            tenant_id,
                            batch_id,
                            "tbl_customers",
                            &staged_customer.member_id,
                            "pppoe_router_mismatch",
                            "Akun PPPoE import sebelumnya terhubung ke router berbeda; perlu review manual.",
                        )
                        .await?;
                        summary.conflict_rows += 1;
                        continue;
                    }
                }

                sqlx::query(
                    r#"
                    UPDATE public.pppoe_accounts
                    SET router_id = $1,
                        customer_id = $2,
                        location_id = $3,
                        username = $4,
                        password_enc = $5,
                        package_id = $6,
                        profile_id = NULL,
                        router_profile_name = NULL,
                        remote_address = $7,
                        address_pool = NULL,
                        disabled = false,
                        comment = $8,
                        account_source = $9,
                        router_present = false,
                        router_secret_id = NULL,
                        last_sync_at = NULL,
                        last_error = NULL,
                        radius_present = false,
                        radius_identity = $4,
                        radius_last_sync_at = NULL,
                        radius_last_error = NULL,
                        updated_at = $10
                    WHERE tenant_id = $11 AND id = $12
                    "#,
                )
                .bind(&router_id)
                .bind(&customer_id)
                .bind(&location_id)
                .bind(&username)
                .bind(&password_enc)
                .bind(&package_id)
                .bind(&remote_address)
                .bind(&notes)
                .bind(account_source)
                .bind(Utc::now())
                .bind(tenant_id)
                .bind(&existing_account_id)
                .execute(&mut *tx)
                .await
                .context("failed to update imported MixRadius PPPoE account")?;

                upsert_external_ref(
                    &mut tx,
                    tenant_id,
                    batch_id,
                    "pppoe_account",
                    &existing_account_id,
                    &staged_customer.member_id,
                )
                .await?;
                if provisioning_target == MixradiusImportPppoeProvisioningTarget::ManagedRadius {
                    pending_managed_radius_sync_ids.push(existing_account_id);
                }
                summary.updated_rows += 1;
                continue;
            }

            let account_id = Uuid::new_v4().to_string();
            sqlx::query(
                r#"
                INSERT INTO public.pppoe_accounts (
                    id,
                    tenant_id,
                    router_id,
                    customer_id,
                    location_id,
                    username,
                    password_enc,
                    package_id,
                    profile_id,
                    router_profile_name,
                    remote_address,
                    address_pool,
                    disabled,
                    comment,
                    account_source,
                    router_present,
                    router_secret_id,
                    last_sync_at,
                    last_error,
                    radius_present,
                    radius_identity,
                    radius_last_sync_at,
                    radius_last_error,
                    created_at,
                    updated_at
                )
                VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, NULL, NULL, $9, NULL, false, $10,
                    $11, false, NULL, NULL, NULL, false, $6, NULL, NULL, $12, $12
                )
                "#,
            )
            .bind(&account_id)
            .bind(tenant_id)
            .bind(&router_id)
            .bind(&customer_id)
            .bind(&location_id)
            .bind(&username)
            .bind(&password_enc)
            .bind(&package_id)
            .bind(&remote_address)
            .bind(&notes)
            .bind(account_source)
            .bind(Utc::now())
            .execute(&mut *tx)
            .await
            .context("failed to create imported MixRadius PPPoE account")?;

            upsert_external_ref(
                &mut tx,
                tenant_id,
                batch_id,
                "pppoe_account",
                &account_id,
                &staged_customer.member_id,
            )
            .await?;
            if provisioning_target == MixradiusImportPppoeProvisioningTarget::ManagedRadius {
                pending_managed_radius_sync_ids.push(account_id);
            }
            summary.imported_rows += 1;
        }

        tx.commit()
            .await
            .context("failed to commit MixRadius PPPoE execution transaction")?;

        if provisioning_target == MixradiusImportPppoeProvisioningTarget::ManagedRadius {
            for account_id in pending_managed_radius_sync_ids {
                if let Err(error) = self
                    .sync_managed_radius_account_after_import(tenant_id, &account_id)
                    .await
                {
                    summary.warnings.push(format!(
                        "Akun PPPoE MixRadius `{account_id}` tersimpan sebagai Managed RADIUS, tetapi belum berhasil dipush ke RADIUS server: {error}"
                    ));
                }
            }
        }

        Ok(summary)
    }

    async fn sync_managed_radius_account_after_import(
        &self,
        tenant_id: &str,
        account_id: &str,
    ) -> Result<()> {
        let mut account: PppoeAccount =
            sqlx::query_as("SELECT * FROM public.pppoe_accounts WHERE tenant_id = $1 AND id = $2")
                .bind(tenant_id)
                .bind(account_id)
                .fetch_optional(&self.pool)
                .await
                .context("failed to load imported managed radius PPPoE account")?
                .ok_or_else(|| anyhow::anyhow!("imported PPPoE account no longer exists"))?;

        let password = decrypt_secret_opt_for(PURPOSE_PPPOE, account.password_enc.as_str())
            .context("failed to decrypt imported PPPoE password for Managed RADIUS sync")?
            .ok_or_else(|| anyhow::anyhow!("imported PPPoE password is empty"))?;

        let now = Utc::now();
        let managed_radius_service = ManagedRadiusService::new(self.pool.clone());
        match managed_radius_service
            .apply_account(tenant_id, &account, password.as_str())
            .await
        {
            Ok(radius) => {
                account.radius_identity = Some(radius.radius_identity);
                sqlx::query(
                    r#"
                    UPDATE public.pppoe_accounts
                    SET router_present = false,
                        router_secret_id = NULL,
                        last_sync_at = $1,
                        last_error = NULL,
                        radius_present = true,
                        radius_identity = $2,
                        radius_last_sync_at = $3,
                        radius_last_error = NULL,
                        updated_at = $4
                    WHERE tenant_id = $5 AND id = $6
                    "#,
                )
                .bind(now)
                .bind(&account.radius_identity)
                .bind(now)
                .bind(now)
                .bind(tenant_id)
                .bind(account_id)
                .execute(&self.pool)
                .await
                .context("failed to mark imported PPPoE account as synced to Managed RADIUS")?;
                Ok(())
            }
            Err(error) => {
                let message = format!("apply failed: {error}");
                sqlx::query(
                    r#"
                    UPDATE public.pppoe_accounts
                    SET radius_present = false,
                        radius_last_sync_at = $1,
                        radius_last_error = $2,
                        last_sync_at = $1,
                        last_error = $2,
                        updated_at = $1
                    WHERE tenant_id = $3 AND id = $4
                    "#,
                )
                .bind(now)
                .bind(&message)
                .bind(tenant_id)
                .bind(account_id)
                .execute(&self.pool)
                .await
                .context("failed to record Managed RADIUS sync error for imported PPPoE account")?;
                Err(anyhow::anyhow!(message))
            }
        }
    }
}

fn normalize_name(input: &str) -> String {
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn same_price(existing_price: f64, source_price: Option<f64>) -> bool {
    source_price
        .map(|price| (existing_price - price).abs() < 0.005)
        .unwrap_or(true)
}

fn build_package_description(plan: &StagedPlanRow) -> String {
    format!(
        "Imported from MixRadius plan `{}` on {}.",
        plan.plan_name.trim(),
        Utc::now().format("%Y-%m-%d")
    )
}

fn build_package_features(plan: &StagedPlanRow) -> Vec<String> {
    let mut features = vec!["PPPoE".to_string(), "Imported from MixRadius".to_string()];
    if let Some(bandwidth_name) = plan
        .bandwidth_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        features.push(format!("Bandwidth: {bandwidth_name}"));
    }
    if let Some(validity) = plan
        .validity
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        features.push(format!("Validity: {validity}"));
    }
    if let Some(shared_users) = plan.shared_users.filter(|value| *value > 0) {
        features.push(format!("Shared users: {shared_users}"));
    }
    features
}

fn build_customer_notes(customer: &StagedCustomerRow) -> String {
    let mut notes = vec!["Imported from MixRadius".to_string()];
    if let Some(username) = normalize_optional(customer.username.as_deref()) {
        notes.push(format!("Username: {username}"));
    }
    if let Some(member_id) = normalize_optional(Some(customer.member_id.as_str())) {
        notes.push(format!("Member ID: {member_id}"));
    }
    notes.join(" | ")
}

fn build_location_notes(customer: &StagedCustomerRow) -> String {
    format!("MixRadius source for member {}.", customer.member_id)
}

fn default_location_source_ref(member_id: &str) -> String {
    format!("customer:{member_id}:default-location")
}

async fn find_imported_location_id_for_member(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    batch_id: &str,
    member_id: &str,
) -> Result<Option<String>> {
    let staged_location_source_ref: Option<String> = sqlx::query_scalar(
        r#"
        SELECT source_ref
        FROM public.mixradius_staging_customer_locations
        WHERE tenant_id = $1
          AND import_batch_id = $2
          AND member_id = $3
        ORDER BY created_at ASC
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(batch_id)
    .bind(member_id)
    .fetch_optional(&mut **tx)
    .await
    .context("failed to resolve MixRadius staged location ref")?;

    let default_ref = default_location_source_ref(member_id);
    let mut candidate_refs = Vec::new();
    if let Some(source_ref) = staged_location_source_ref
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        candidate_refs.push(source_ref);
    }
    if !candidate_refs
        .iter()
        .any(|source_ref| source_ref == &default_ref)
    {
        candidate_refs.push(default_ref);
    }

    for source_ref in candidate_refs {
        if let Some(location_id) =
            find_external_ref_entity_id(tx, tenant_id, "location", &source_ref).await?
        {
            return Ok(Some(location_id));
        }
    }

    Ok(None)
}

fn resolve_billing_cycle(staged_customer: &StagedCustomerRow) -> &'static str {
    match staged_customer
        .plan_name
        .as_deref()
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some(value) if value.contains("tahun") || value.contains("year") => "yearly",
        _ => "monthly",
    }
}

fn extract_framed_ip_address(source_json: &Value) -> Option<String> {
    source_json
        .get("radreply")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find_map(|item| {
            let attribute = item.get("attribute").and_then(Value::as_str)?.trim();
            if !attribute.eq_ignore_ascii_case("Framed-IP-Address") {
                return None;
            }

            item.get("value")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
}

fn find_override_target<'a>(
    overrides: &'a [MixradiusImportMappingOverride],
    source_kind: &str,
    source_value: &str,
) -> Option<&'a str> {
    overrides
        .iter()
        .find(|item| {
            item.source_kind == source_kind
                && item.source_value == source_value
                && item.target_kind == "package"
        })
        .map(|item| item.target_value.as_str())
}

async fn upsert_package_router_mapping_for_mixradius_plan(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    router_id: Option<&str>,
    package_id: &str,
    plan: &StagedPlanRow,
    summary: &mut MixradiusPackageExecutionSummary,
) -> Result<()> {
    let Some(router_id) = router_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };

    let mapping_table_exists: bool =
        sqlx::query_scalar("SELECT to_regclass('public.isp_package_router_mappings') IS NOT NULL")
            .fetch_one(&mut **tx)
            .await
            .context("failed to inspect package-router mapping table for MixRadius import")?;
    if !mapping_table_exists {
        return Ok(());
    }

    let profile_table_exists: bool =
        sqlx::query_scalar("SELECT to_regclass('public.mikrotik_ppp_profiles') IS NOT NULL")
            .fetch_one(&mut **tx)
            .await
            .context("failed to inspect MikroTik PPP profile table for MixRadius import")?;
    let desired_profile_name = plan.plan_name.trim();
    if desired_profile_name.is_empty() {
        return Ok(());
    }

    let router_profile_name = if profile_table_exists {
        let exact_profile_name: Option<String> = sqlx::query_scalar(
            r#"
            SELECT name
            FROM public.mikrotik_ppp_profiles
            WHERE tenant_id = $1
              AND router_id = $2
              AND router_present = TRUE
              AND lower(trim(name)) = lower($3)
            ORDER BY name ASC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .bind(desired_profile_name)
        .fetch_optional(&mut **tx)
        .await
        .context("failed to resolve router profile for MixRadius package mapping")?;

        if let Some(exact_profile_name) = exact_profile_name {
            exact_profile_name
        } else if let Some(bandwidth_profile_name) =
            resolve_bandwidth_matched_router_profile(tx, tenant_id, router_id, plan).await?
        {
            bandwidth_profile_name
        } else {
            summary.warnings.push(format!(
                "Package `{}` dibuat dan mapping package-router tetap dibuat memakai profile `{}`, tetapi profile tersebut belum ada pada router terpilih. Sinkronkan atau buat profile di router agar auto provisioning berjalan mulus.",
                plan.plan_name.trim(),
                desired_profile_name
            ));
            desired_profile_name.to_string()
        }
    } else {
        summary.warnings.push(format!(
            "Package `{}` dibuat dan mapping package-router tetap dibuat memakai profile `{}`, tetapi inventory profile router belum tersedia. Sinkronkan profile router agar auto provisioning tervalidasi.",
            plan.plan_name.trim(),
            desired_profile_name
        ));
        desired_profile_name.to_string()
    };

    let now = Utc::now();
    sqlx::query(
        r#"
        INSERT INTO public.isp_package_router_mappings (
            id,
            tenant_id,
            router_id,
            package_id,
            router_profile_name,
            address_pool,
            created_at,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, NULL, $6, $6)
        ON CONFLICT (tenant_id, router_id, package_id) DO UPDATE SET
            router_profile_name = EXCLUDED.router_profile_name,
            updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(tenant_id)
    .bind(router_id)
    .bind(package_id)
    .bind(router_profile_name)
    .bind(now)
    .execute(&mut **tx)
    .await
    .context("failed to upsert MixRadius package router mapping")?;

    Ok(())
}

async fn resolve_bandwidth_matched_router_profile(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    router_id: &str,
    plan: &StagedPlanRow,
) -> Result<Option<String>> {
    let desired_bandwidths = plan_bandwidth_candidates(plan);
    if desired_bandwidths.is_empty() {
        return Ok(None);
    }

    let profiles = sqlx::query_as::<_, RouterProfileCandidateRow>(
        r#"
        SELECT name, rate_limit
        FROM public.mikrotik_ppp_profiles
        WHERE tenant_id = $1
          AND router_id = $2
          AND router_present = TRUE
        ORDER BY name ASC
        "#,
    )
    .bind(tenant_id)
    .bind(router_id)
    .fetch_all(&mut **tx)
    .await
    .context("failed to load router profiles for MixRadius bandwidth mapping")?;

    let mut best_match: Option<(i32, String)> = None;
    for profile in profiles {
        let name_bandwidths = extract_mbps_candidates(profile.name.as_str());
        let rate_bandwidths = extract_rate_limit_mbps_candidates(profile.rate_limit.as_deref());

        let score = desired_bandwidths
            .iter()
            .map(|desired| {
                let name_match = name_bandwidths.contains(desired);
                let rate_match = rate_bandwidths.contains(desired);
                match (name_match, rate_match) {
                    (true, true) => 3,
                    (true, false) => 2,
                    (false, true) => 1,
                    (false, false) => 0,
                }
            })
            .max()
            .unwrap_or(0);

        if score == 0 {
            continue;
        }

        match &best_match {
            Some((best_score, best_name)) if *best_score > score => {}
            Some((best_score, best_name)) if *best_score == score && best_name <= &profile.name => {
            }
            _ => best_match = Some((score, profile.name)),
        }
    }

    Ok(best_match.map(|(_, name)| name))
}

fn plan_bandwidth_candidates(plan: &StagedPlanRow) -> Vec<u32> {
    let mut values = extract_mbps_candidates(&plan.plan_name);
    if let Some(bandwidth_name) = plan.bandwidth_name.as_deref() {
        values.extend(extract_mbps_candidates(bandwidth_name));
    }
    values.sort_unstable();
    values.dedup();
    values
}

fn extract_mbps_candidates(value: &str) -> Vec<u32> {
    extract_numeric_token_before_marker(value, "mbps")
}

fn extract_rate_limit_mbps_candidates(value: Option<&str>) -> Vec<u32> {
    value
        .map(|text| extract_numeric_token_before_marker(text, "m/"))
        .unwrap_or_default()
}

fn extract_numeric_token_before_marker(value: &str, marker: &str) -> Vec<u32> {
    let lower = value.to_ascii_lowercase();
    let mut results = Vec::new();
    let mut search_start = 0usize;

    while let Some(found_at) = lower[search_start..].find(marker) {
        let marker_start = search_start + found_at;
        let digits_end = marker_start;
        let digits_start = lower[..digits_end]
            .rfind(|ch: char| !ch.is_ascii_digit())
            .map(|index| index + 1)
            .unwrap_or(0);

        if digits_start < digits_end {
            if let Ok(parsed) = lower[digits_start..digits_end].parse::<u32>() {
                if !results.contains(&parsed) {
                    results.push(parsed);
                }
            }
        }

        search_start = marker_start + marker.len();
    }

    results
}

async fn resolve_mixradius_router_id(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    batch_id: &str,
    mapping_overrides: &[MixradiusImportMappingOverride],
) -> Result<Option<String>> {
    let router_table_exists: bool =
        sqlx::query_scalar("SELECT to_regclass('public.mikrotik_routers') IS NOT NULL")
            .fetch_one(&mut **tx)
            .await
            .context("failed to inspect MikroTik router table for MixRadius PPPoE import")?;
    if !router_table_exists {
        return Ok(None);
    }

    let nas_refs = sqlx::query_scalar::<_, String>(
        r#"
        SELECT source_ref
        FROM public.mixradius_staging_nas
        WHERE tenant_id = $1 AND import_batch_id = $2
        ORDER BY created_at ASC
        "#,
    )
    .bind(tenant_id)
    .bind(batch_id)
    .fetch_all(&mut **tx)
    .await
    .context("failed to load staged MixRadius NAS rows for PPPoE router resolution")?;

    for nas_ref in nas_refs {
        if let Some(router_id) = mapping_overrides
            .iter()
            .find(|item| {
                item.target_kind == "router"
                    && (item.source_kind == "nas" || item.source_kind == "router")
                    && item.source_value == nas_ref
            })
            .map(|item| item.target_value.clone())
        {
            let exists: Option<String> = sqlx::query_scalar(
                "SELECT id FROM public.mikrotik_routers WHERE tenant_id = $1 AND id = $2",
            )
            .bind(tenant_id)
            .bind(&router_id)
            .fetch_optional(&mut **tx)
            .await
            .context("failed to validate MixRadius PPPoE router target")?;
            if exists.is_some() {
                return Ok(Some(router_id));
            }
        }
    }

    Ok(None)
}

fn normalize_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

async fn insert_customer(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    staged_customer: &StagedCustomerRow,
    customer_source_ref: &str,
    batch_id: &str,
) -> Result<String> {
    let customer_id = Uuid::new_v4().to_string();
    let customer_name = normalize_optional(staged_customer.fullname.as_deref())
        .or_else(|| normalize_optional(staged_customer.username.as_deref()))
        .unwrap_or_else(|| staged_customer.member_id.clone());

    sqlx::query(
        r#"
        INSERT INTO public.customers (
            id,
            tenant_id,
            name,
            email,
            phone,
            notes,
            is_active,
            created_at,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, true, $7, $7)
        "#,
    )
    .bind(&customer_id)
    .bind(tenant_id)
    .bind(customer_name)
    .bind(normalize_optional(staged_customer.email.as_deref()))
    .bind(normalize_optional(staged_customer.phonenumber.as_deref()))
    .bind(build_customer_notes(staged_customer))
    .bind(Utc::now())
    .execute(&mut **tx)
    .await
    .context("failed to insert imported MixRadius customer")?;

    upsert_external_ref(
        tx,
        tenant_id,
        batch_id,
        "customer",
        &customer_id,
        customer_source_ref,
    )
    .await?;

    Ok(customer_id)
}

async fn create_location_for_customer(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    batch_id: &str,
    customer_id: &str,
    location_source_ref: &str,
    address_line1: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    notes: String,
) -> Result<String> {
    let location_id = Uuid::new_v4().to_string();

    sqlx::query(
        r#"
        INSERT INTO public.customer_locations (
            id,
            tenant_id,
            customer_id,
            label,
            address_line1,
            address_line2,
            city,
            state,
            postal_code,
            country,
            latitude,
            longitude,
            notes,
            created_at,
            updated_at
        )
        VALUES (
            $1, $2, $3, 'Lokasi Utama', $4, NULL, NULL, NULL, NULL, NULL, $5, $6, $7, $8, $8
        )
        "#,
    )
    .bind(&location_id)
    .bind(tenant_id)
    .bind(customer_id)
    .bind(address_line1)
    .bind(latitude)
    .bind(longitude)
    .bind(notes)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await
    .context("failed to insert imported MixRadius customer location")?;

    upsert_external_ref(
        tx,
        tenant_id,
        batch_id,
        "location",
        &location_id,
        location_source_ref,
    )
    .await?;

    Ok(location_id)
}

async fn find_external_ref_entity_id(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    entity_type: &str,
    source_ref: &str,
) -> Result<Option<String>> {
    sqlx::query_scalar::<_, String>(
        r#"
        SELECT entity_id
        FROM public.mixradius_import_external_refs
        WHERE tenant_id = $1
          AND source_system = 'mixradius'
          AND entity_type = $2
          AND source_ref = $3
        "#,
    )
    .bind(tenant_id)
    .bind(entity_type)
    .bind(source_ref)
    .fetch_optional(&mut **tx)
    .await
    .context("failed to load MixRadius external ref")
}

async fn find_package_id_for_customer(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    staged_customer: &StagedCustomerRow,
) -> Result<Option<String>> {
    let Some(plan_name) = staged_customer.plan_name.as_deref() else {
        return Ok(None);
    };

    let packages = sqlx::query_as::<_, ExistingPackageRow>(
        r#"
        SELECT id, name, price_monthly::float8 AS price_monthly
        FROM public.isp_packages
        WHERE tenant_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(&mut **tx)
    .await
    .context("failed to load packages for MixRadius subscription resolution")?;

    let normalized_plan_name = normalize_name(plan_name);
    Ok(packages
        .into_iter()
        .find(|package| {
            normalize_name(&package.name) == normalized_plan_name
                && same_price(package.price_monthly, staged_customer.price)
        })
        .map(|package| package.id))
}

async fn find_latest_location_package_id(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    location_id: &str,
) -> Result<Option<String>> {
    sqlx::query_scalar::<_, String>(
        r#"
        SELECT package_id
        FROM public.customer_subscriptions
        WHERE tenant_id = $1
          AND location_id = $2
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(location_id)
    .fetch_optional(&mut **tx)
    .await
    .context("failed to resolve latest package for imported PPPoE account")
}

async fn insert_generic_conflict(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    batch_id: &str,
    source_table: &str,
    source_ref: &str,
    conflict_type: &str,
    conflict_message: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO public.mixradius_import_conflicts (
            id,
            tenant_id,
            import_batch_id,
            source_table,
            source_ref,
            conflict_type,
            severity,
            conflict_message,
            resolution_status,
            details_json,
            created_at,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 'warning', $7, 'open', '{}'::jsonb, $8, $8)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(tenant_id)
    .bind(batch_id)
    .bind(source_table)
    .bind(source_ref)
    .bind(conflict_type)
    .bind(conflict_message)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await
    .context("failed to insert MixRadius generic conflict")?;

    Ok(())
}

async fn upsert_external_ref(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    batch_id: &str,
    entity_type: &str,
    entity_id: &str,
    source_ref: &str,
) -> Result<()> {
    let now = Utc::now();
    sqlx::query(
        r#"
        INSERT INTO public.mixradius_import_external_refs (
            id,
            tenant_id,
            import_batch_id,
            entity_type,
            entity_id,
            source_system,
            source_ref,
            last_seen_at,
            created_at,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, 'mixradius', $6, $7, $7, $7)
        ON CONFLICT (tenant_id, source_system, entity_type, source_ref)
        DO UPDATE SET
            import_batch_id = EXCLUDED.import_batch_id,
            entity_id = EXCLUDED.entity_id,
            last_seen_at = EXCLUDED.last_seen_at,
            updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(tenant_id)
    .bind(batch_id)
    .bind(entity_type)
    .bind(entity_id)
    .bind(source_ref)
    .bind(now)
    .execute(&mut **tx)
    .await
    .context("failed to upsert MixRadius external ref")?;

    Ok(())
}

async fn insert_conflict(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    batch_id: &str,
    source_ref: &str,
    conflict_type: &str,
    conflict_message: &str,
    details_json: serde_json::Value,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO public.mixradius_import_conflicts (
            id,
            tenant_id,
            import_batch_id,
            source_table,
            source_ref,
            conflict_type,
            severity,
            conflict_message,
            resolution_status,
            details_json,
            created_at,
            updated_at
        )
        VALUES ($1, $2, $3, 'tbl_plans', $4, $5, 'warning', $6, 'open', $7, $8, $8)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(tenant_id)
    .bind(batch_id)
    .bind(source_ref)
    .bind(conflict_type)
    .bind(conflict_message)
    .bind(details_json)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await
    .context("failed to insert MixRadius import conflict")?;

    Ok(())
}
