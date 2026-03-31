use super::*;

impl CustomerService {
    pub async fn get_pending_work_order_reschedule_request(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
    ) -> AppResult<Option<WorkOrderRescheduleRequestView>> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "read")
            .await?;

        #[cfg(feature = "postgres")]
        let row: Option<WorkOrderRescheduleRequestView> = sqlx::query_as(
            r#"
            SELECT
              r.id,
              r.work_order_id,
              CAST(r.requested_schedule_at AS TEXT) AS requested_schedule_at,
              r.reason,
              r.status,
              req.name AS requested_by_name,
              req.email AS requested_by_email,
              rev.name AS reviewed_by_name,
              CAST(r.reviewed_at AS TEXT) AS reviewed_at,
              r.review_notes,
              CAST(r.created_at AS TEXT) AS created_at
            FROM work_order_reschedule_requests r
            LEFT JOIN users req ON req.id = r.requested_by
            LEFT JOIN users rev ON rev.id = r.reviewed_by
            WHERE r.tenant_id = $1
              AND r.work_order_id = $2
              AND r.status = 'pending'
            ORDER BY r.created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(work_order_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let row: Option<WorkOrderRescheduleRequestView> = sqlx::query_as(
            r#"
            SELECT
              r.id,
              r.work_order_id,
              CAST(r.requested_schedule_at AS TEXT) AS requested_schedule_at,
              r.reason,
              r.status,
              req.name AS requested_by_name,
              req.email AS requested_by_email,
              rev.name AS reviewed_by_name,
              CAST(r.reviewed_at AS TEXT) AS reviewed_at,
              r.review_notes,
              CAST(r.created_at AS TEXT) AS created_at
            FROM work_order_reschedule_requests r
            LEFT JOIN users req ON req.id = r.requested_by
            LEFT JOIN users rev ON rev.id = r.reviewed_by
            WHERE r.tenant_id = ?
              AND r.work_order_id = ?
              AND r.status = 'pending'
            ORDER BY r.created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(work_order_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn approve_work_order_reschedule_request(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
        dto: WorkOrderRescheduleDecisionRequest,
        ip_address: Option<&str>,
    ) -> AppResult<InstallationWorkOrder> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await?;
        let is_admin_owner = self.is_actor_admin_or_owner(tenant_id, actor_id).await?;
        let current = self
            .get_installation_work_order_row(tenant_id, work_order_id)
            .await?;
        let is_assigned_technician = current
            .assigned_to
            .as_deref()
            .map(str::trim)
            .map(|v| v == actor_id)
            .unwrap_or(false);
        if !is_admin_owner && !is_assigned_technician {
            return Err(AppError::Forbidden(
                "Only admin/owner or assigned technician can approve reschedule request"
                    .to_string(),
            ));
        }

        let pending = self
            .get_pending_work_order_reschedule_request(actor_id, tenant_id, work_order_id)
            .await?
            .ok_or_else(|| AppError::NotFound("No pending reschedule request".to_string()))?;

        let target_schedule = dto
            .scheduled_at
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| pending.requested_schedule_at.clone());

        let note = format!(
            "Reschedule approved. New schedule: {}{}",
            target_schedule,
            dto.notes
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(|v| format!(". Notes: {}", v))
                .unwrap_or_default()
        );

        let row = self
            .set_installation_work_order_status_internal(
                actor_id,
                tenant_id,
                work_order_id,
                Some("pending"),
                None,
                Some(target_schedule),
                Some(note),
                false,
                ip_address,
                "WORK_ORDER_RESCHEDULE_APPROVE",
                "Approved work order reschedule request",
            )
            .await?;

        let now = Utc::now();
        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            UPDATE work_order_reschedule_requests
            SET status = 'approved',
                reviewed_by = $1,
                reviewed_at = $2,
                review_notes = $3,
                updated_at = $2
            WHERE tenant_id = $4
              AND id = $5
            "#,
        )
        .bind(actor_id)
        .bind(now)
        .bind(dto.notes)
        .bind(tenant_id)
        .bind(&pending.id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            UPDATE work_order_reschedule_requests
            SET status = 'approved',
                reviewed_by = ?,
                reviewed_at = ?,
                review_notes = ?,
                updated_at = ?
            WHERE tenant_id = ?
              AND id = ?
            "#,
        )
        .bind(actor_id)
        .bind(now.to_rfc3339())
        .bind(dto.notes)
        .bind(now.to_rfc3339())
        .bind(tenant_id)
        .bind(&pending.id)
        .execute(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn reject_work_order_reschedule_request(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
        dto: WorkOrderRescheduleDecisionRequest,
        ip_address: Option<&str>,
    ) -> AppResult<InstallationWorkOrder> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await?;
        let is_admin_owner = self.is_actor_admin_or_owner(tenant_id, actor_id).await?;
        let current = self
            .get_installation_work_order_row(tenant_id, work_order_id)
            .await?;
        let is_assigned_technician = current
            .assigned_to
            .as_deref()
            .map(str::trim)
            .map(|v| v == actor_id)
            .unwrap_or(false);
        if !is_admin_owner && !is_assigned_technician {
            return Err(AppError::Forbidden(
                "Only admin/owner or assigned technician can reject reschedule request".to_string(),
            ));
        }

        let pending = self
            .get_pending_work_order_reschedule_request(actor_id, tenant_id, work_order_id)
            .await?
            .ok_or_else(|| AppError::NotFound("No pending reschedule request".to_string()))?;

        let reason = dto
            .notes
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| AppError::Validation("Rejection reason is required".to_string()))?;

        let row = self
            .set_installation_work_order_status_internal(
                actor_id,
                tenant_id,
                work_order_id,
                None,
                None,
                None,
                Some(format!("Reschedule request rejected. Reason: {}", reason)),
                false,
                ip_address,
                "WORK_ORDER_RESCHEDULE_REJECT",
                "Rejected work order reschedule request",
            )
            .await?;

        let now = Utc::now();
        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            UPDATE work_order_reschedule_requests
            SET status = 'rejected',
                reviewed_by = $1,
                reviewed_at = $2,
                review_notes = $3,
                updated_at = $2
            WHERE tenant_id = $4
              AND id = $5
            "#,
        )
        .bind(actor_id)
        .bind(now)
        .bind(Some(reason.to_string()))
        .bind(tenant_id)
        .bind(&pending.id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            UPDATE work_order_reschedule_requests
            SET status = 'rejected',
                reviewed_by = ?,
                reviewed_at = ?,
                review_notes = ?,
                updated_at = ?
            WHERE tenant_id = ?
              AND id = ?
            "#,
        )
        .bind(actor_id)
        .bind(now.to_rfc3339())
        .bind(Some(reason.to_string()))
        .bind(now.to_rfc3339())
        .bind(tenant_id)
        .bind(&pending.id)
        .execute(&self.pool)
        .await?;

        Ok(row)
    }
}
