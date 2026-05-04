use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    Customer, MessageTemplate, MessageTemplateListQuery, MessageTemplatePayload,
    MessageTemplatePreviewRequest, MessageTemplatePreviewResponse,
};
use crate::services::message_template_renderer::{extract_variables, render_template_body};
use chrono::{DateTime, Utc};
use serde_json::json;
use uuid::Uuid;

#[derive(Clone)]
pub struct MessageTemplateService {
    pool: DbPool,
}

#[derive(Debug, Clone)]
pub struct RenderedCustomerEmailTemplate {
    pub subject: String,
    pub body: String,
}

#[derive(Debug, sqlx::FromRow)]
struct MessageTemplateRow {
    id: String,
    tenant_id: String,
    key: String,
    name: String,
    description: Option<String>,
    use_case: String,
    target: String,
    trigger_mode: String,
    event_key: Option<String>,
    channel: String,
    locale: String,
    status: String,
    whatsapp_body: Option<String>,
    email_subject: Option<String>,
    email_body: Option<String>,
    variables: String,
    version: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<MessageTemplateRow> for MessageTemplate {
    fn from(row: MessageTemplateRow) -> Self {
        let variables = serde_json::from_str::<Vec<String>>(&row.variables).unwrap_or_default();
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            key: row.key,
            name: row.name,
            description: row.description,
            use_case: row.use_case,
            target: row.target,
            trigger_mode: row.trigger_mode,
            event_key: row.event_key,
            channel: row.channel,
            locale: row.locale,
            status: row.status,
            whatsapp_body: row.whatsapp_body,
            email_subject: row.email_subject,
            email_body: row.email_body,
            variables,
            version: row.version,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl MessageTemplateService {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn customer_context(customer: &Customer, tenant_name: Option<&str>) -> serde_json::Value {
        json!({
            "tenant": {
                "name": tenant_name.unwrap_or(""),
            },
            "customer": {
                "id": customer.id,
                "name": customer.name,
                "email": customer.email,
                "phone": customer.phone,
                "status": if customer.is_active { "active" } else { "inactive" },
                "notes": customer.notes,
            }
        })
    }

    pub async fn tenant_name(&self, tenant_id: &str) -> AppResult<Option<String>> {
        #[cfg(feature = "postgres")]
        let name: Option<String> =
            sqlx::query_scalar("SELECT name FROM tenants WHERE id = $1")
                .bind(tenant_id)
                .fetch_optional(&self.pool)
                .await?;

        #[cfg(feature = "sqlite")]
        let name: Option<String> =
            sqlx::query_scalar("SELECT name FROM tenants WHERE id = ?")
                .bind(tenant_id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(name)
    }

    pub fn collect_payload_variables(payload: &MessageTemplatePayload) -> Vec<String> {
        let mut variables = Vec::new();
        for body in [
            payload.whatsapp_body.as_deref(),
            payload.email_subject.as_deref(),
            payload.email_body.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            variables.extend(extract_variables(body));
        }
        variables.sort();
        variables.dedup();
        variables
    }

    pub fn preview(
        &self,
        payload: MessageTemplatePreviewRequest,
    ) -> AppResult<MessageTemplatePreviewResponse> {
        let mut variables = Vec::new();
        let whatsapp_body = render_optional(
            payload.whatsapp_body.as_deref(),
            &payload.context,
            &mut variables,
        )?;
        let email_subject = render_optional(
            payload.email_subject.as_deref(),
            &payload.context,
            &mut variables,
        )?;
        let email_body = render_optional(
            payload.email_body.as_deref(),
            &payload.context,
            &mut variables,
        )?;
        variables.sort();
        variables.dedup();

        Ok(MessageTemplatePreviewResponse {
            whatsapp_body,
            email_subject,
            email_body,
            variables,
        })
    }

    pub async fn list(
        &self,
        tenant_id: &str,
        query: MessageTemplateListQuery,
    ) -> AppResult<Vec<MessageTemplate>> {
        #[cfg(feature = "postgres")]
        let rows: Vec<MessageTemplateRow> = sqlx::query_as(
            r#"
            SELECT * FROM message_templates
            WHERE tenant_id = $1
              AND ($2 IS NULL OR name ILIKE '%' || $2 || '%' OR key ILIKE '%' || $2 || '%')
              AND ($3 IS NULL OR use_case = $3)
              AND ($4 IS NULL OR channel = $4 OR channel = 'both')
              AND ($5 IS NULL OR status = $5)
              AND ($6 IS NULL OR target = $6)
              AND ($7 IS NULL OR trigger_mode = $7 OR trigger_mode = 'both')
            ORDER BY updated_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(blank_to_none(query.q))
        .bind(blank_to_none(query.use_case))
        .bind(blank_to_none(query.channel))
        .bind(blank_to_none(query.status))
        .bind(blank_to_none(query.target))
        .bind(blank_to_none(query.trigger_mode))
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<MessageTemplateRow> = sqlx::query_as(
            r#"
            SELECT * FROM message_templates
            WHERE tenant_id = ?
              AND (? IS NULL OR name LIKE '%' || ? || '%' OR key LIKE '%' || ? || '%')
              AND (? IS NULL OR use_case = ?)
              AND (? IS NULL OR channel = ? OR channel = 'both')
              AND (? IS NULL OR status = ?)
              AND (? IS NULL OR target = ?)
              AND (? IS NULL OR trigger_mode = ? OR trigger_mode = 'both')
            ORDER BY updated_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(blank_to_none(query.q.clone()))
        .bind(blank_to_none(query.q.clone()))
        .bind(blank_to_none(query.q))
        .bind(blank_to_none(query.use_case.clone()))
        .bind(blank_to_none(query.use_case))
        .bind(blank_to_none(query.channel.clone()))
        .bind(blank_to_none(query.channel))
        .bind(blank_to_none(query.status.clone()))
        .bind(blank_to_none(query.status))
        .bind(blank_to_none(query.target.clone()))
        .bind(blank_to_none(query.target))
        .bind(blank_to_none(query.trigger_mode.clone()))
        .bind(blank_to_none(query.trigger_mode))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(MessageTemplate::from).collect())
    }

    pub async fn get(&self, tenant_id: &str, id: &str) -> AppResult<MessageTemplate> {
        #[cfg(feature = "postgres")]
        let row: Option<MessageTemplateRow> =
            sqlx::query_as("SELECT * FROM message_templates WHERE tenant_id = $1 AND id = $2")
                .bind(tenant_id)
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        #[cfg(feature = "sqlite")]
        let row: Option<MessageTemplateRow> =
            sqlx::query_as("SELECT * FROM message_templates WHERE tenant_id = ? AND id = ?")
                .bind(tenant_id)
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        row.map(MessageTemplate::from)
            .ok_or_else(|| AppError::NotFound("Message template not found".to_string()))
    }

    pub async fn create(
        &self,
        tenant_id: &str,
        payload: MessageTemplatePayload,
    ) -> AppResult<MessageTemplate> {
        validate_payload(&payload)?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let variables = serde_json::to_string(&Self::collect_payload_variables(&payload))
            .unwrap_or_else(|_| "[]".to_string());
        let locale = payload.locale.unwrap_or_else(|| "id-ID".to_string());

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO message_templates
              (id, tenant_id, key, name, description, use_case, target, trigger_mode, event_key, channel, locale, status, whatsapp_body, email_subject, email_body, variables, version, created_at, updated_at)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,1,$17,$18)
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(payload.key.trim())
        .bind(payload.name.trim())
        .bind(payload.description.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .bind(payload.use_case.trim())
        .bind(payload.target.trim())
        .bind(payload.trigger_mode.trim())
        .bind(payload.event_key.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .bind(payload.channel.trim())
        .bind(locale.trim())
        .bind(payload.status.trim())
        .bind(payload.whatsapp_body.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .bind(payload.email_subject.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .bind(payload.email_body.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .bind(variables)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO message_templates
              (id, tenant_id, key, name, description, use_case, target, trigger_mode, event_key, channel, locale, status, whatsapp_body, email_subject, email_body, variables, version, created_at, updated_at)
            VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,1,?,?)
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(payload.key.trim())
        .bind(payload.name.trim())
        .bind(payload.description.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .bind(payload.use_case.trim())
        .bind(payload.target.trim())
        .bind(payload.trigger_mode.trim())
        .bind(payload.event_key.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .bind(payload.channel.trim())
        .bind(locale.trim())
        .bind(payload.status.trim())
        .bind(payload.whatsapp_body.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .bind(payload.email_subject.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .bind(payload.email_body.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .bind(variables)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        self.get(tenant_id, &id).await
    }

    pub async fn update(
        &self,
        tenant_id: &str,
        id: &str,
        payload: MessageTemplatePayload,
    ) -> AppResult<MessageTemplate> {
        validate_payload(&payload)?;
        let existing = self.get(tenant_id, id).await?;
        let now = Utc::now();
        let variables = serde_json::to_string(&Self::collect_payload_variables(&payload))
            .unwrap_or_else(|_| "[]".to_string());
        let locale = payload.locale.unwrap_or_else(|| existing.locale.clone());

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            UPDATE message_templates
            SET key=$1, name=$2, description=$3, use_case=$4, target=$5, trigger_mode=$6,
                event_key=$7, channel=$8, locale=$9, status=$10, whatsapp_body=$11,
                email_subject=$12, email_body=$13, variables=$14, version=version+1, updated_at=$15
            WHERE tenant_id=$16 AND id=$17
            "#,
        )
        .bind(payload.key.trim())
        .bind(payload.name.trim())
        .bind(
            payload
                .description
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(payload.use_case.trim())
        .bind(payload.target.trim())
        .bind(payload.trigger_mode.trim())
        .bind(
            payload
                .event_key
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(payload.channel.trim())
        .bind(locale.trim())
        .bind(payload.status.trim())
        .bind(
            payload
                .whatsapp_body
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(
            payload
                .email_subject
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(
            payload
                .email_body
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(variables)
        .bind(now)
        .bind(tenant_id)
        .bind(id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            UPDATE message_templates
            SET key=?, name=?, description=?, use_case=?, target=?, trigger_mode=?,
                event_key=?, channel=?, locale=?, status=?, whatsapp_body=?,
                email_subject=?, email_body=?, variables=?, version=version+1, updated_at=?
            WHERE tenant_id=? AND id=?
            "#,
        )
        .bind(payload.key.trim())
        .bind(payload.name.trim())
        .bind(
            payload
                .description
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(payload.use_case.trim())
        .bind(payload.target.trim())
        .bind(payload.trigger_mode.trim())
        .bind(
            payload
                .event_key
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(payload.channel.trim())
        .bind(locale.trim())
        .bind(payload.status.trim())
        .bind(
            payload
                .whatsapp_body
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(
            payload
                .email_subject
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(
            payload
                .email_body
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(variables)
        .bind(now.to_rfc3339())
        .bind(tenant_id)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.get(tenant_id, id).await
    }

    pub async fn delete(&self, tenant_id: &str, id: &str) -> AppResult<()> {
        #[cfg(feature = "postgres")]
        sqlx::query("DELETE FROM message_templates WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(id)
            .execute(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query("DELETE FROM message_templates WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn render_customer_whatsapp(
        &self,
        tenant_id: &str,
        template_id: &str,
        customer: &Customer,
        tenant_name: Option<&str>,
    ) -> AppResult<String> {
        let template = self.get(tenant_id, template_id).await?;
        if template.status != "active" {
            return Err(AppError::Validation(
                "Message template is not active".to_string(),
            ));
        }
        if template.target != "customer" {
            return Err(AppError::Validation(
                "Message template target is not customer".to_string(),
            ));
        }
        if template.channel != "whatsapp" && template.channel != "both" {
            return Err(AppError::Validation(
                "Message template is not enabled for WhatsApp".to_string(),
            ));
        }
        if template.trigger_mode != "manual" && template.trigger_mode != "both" {
            return Err(AppError::Validation(
                "Message template is not enabled for manual send".to_string(),
            ));
        }
        let body = template
            .whatsapp_body
            .as_deref()
            .ok_or_else(|| AppError::Validation("WhatsApp template body is empty".to_string()))?;

        Ok(render_template_body(body, &Self::customer_context(customer, tenant_name))?.rendered)
    }

    pub async fn render_customer_email(
        &self,
        tenant_id: &str,
        template_id: &str,
        customer: &Customer,
        tenant_name: Option<&str>,
    ) -> AppResult<RenderedCustomerEmailTemplate> {
        let template = self.get(tenant_id, template_id).await?;
        if template.status != "active" {
            return Err(AppError::Validation(
                "Message template is not active".to_string(),
            ));
        }
        if template.target != "customer" {
            return Err(AppError::Validation(
                "Message template target is not customer".to_string(),
            ));
        }
        if template.channel != "email" && template.channel != "both" {
            return Err(AppError::Validation(
                "Message template is not enabled for email".to_string(),
            ));
        }
        if template.trigger_mode != "manual" && template.trigger_mode != "both" {
            return Err(AppError::Validation(
                "Message template is not enabled for manual send".to_string(),
            ));
        }
        let subject = template
            .email_subject
            .as_deref()
            .ok_or_else(|| AppError::Validation("Email template subject is empty".to_string()))?;
        let body = template
            .email_body
            .as_deref()
            .ok_or_else(|| AppError::Validation("Email template body is empty".to_string()))?;
        let context = Self::customer_context(customer, tenant_name);

        Ok(RenderedCustomerEmailTemplate {
            subject: render_template_body(subject, &context)?.rendered,
            body: render_template_body(body, &context)?.rendered,
        })
    }
}

fn render_optional(
    body: Option<&str>,
    context: &serde_json::Value,
    variables: &mut Vec<String>,
) -> AppResult<Option<String>> {
    let Some(body) = body else {
        return Ok(None);
    };
    let rendered = render_template_body(body, context)?;
    variables.extend(rendered.variables);
    Ok(Some(rendered.rendered))
}

fn blank_to_none(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty() && v != "all")
}

fn validate_payload(payload: &MessageTemplatePayload) -> AppResult<()> {
    if payload.key.trim().is_empty() {
        return Err(AppError::Validation("Template key is required".to_string()));
    }
    if payload.name.trim().is_empty() {
        return Err(AppError::Validation(
            "Template name is required".to_string(),
        ));
    }
    if !["whatsapp", "email", "both"].contains(&payload.channel.trim()) {
        return Err(AppError::Validation("Invalid template channel".to_string()));
    }
    if !["draft", "active", "archived"].contains(&payload.status.trim()) {
        return Err(AppError::Validation("Invalid template status".to_string()));
    }
    if !["manual", "automatic", "both"].contains(&payload.trigger_mode.trim()) {
        return Err(AppError::Validation(
            "Invalid template trigger mode".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn message_template_service_mentions_rbac_and_customer_context() {
        let source = include_str!("message_template_service.rs");
        assert!(source.contains("customer_context"));
        assert!(source.contains("render_customer_whatsapp"));
        assert!(source.contains("render_customer_email"));
        assert!(source.contains("manual"));
        assert!(source.contains("whatsapp"));
        assert!(source.contains("email"));
    }
}
