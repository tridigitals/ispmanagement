use crate::http::auth::extract_ip;
use crate::http::AppState;
use crate::services::storage_service::StorageContent;
use axum::{
    body::Body,
    extract::{ConnectInfo, Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::net::SocketAddr;
use tokio::fs;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_util::io::ReaderStream;
use tracing::{error, info, warn};

#[derive(serde::Deserialize, Default)]
pub struct FileAccessQuery {
    /// Optional token for cases where the browser can't set `Authorization` header
    /// (e.g. `<img src>`, `<video src>`, `<a href>`).
    pub token: Option<String>,
}

/// Infer MIME type from file extension when the multipart field doesn't provide one.
///
/// Many mobile pickers (image_picker, file_picker) don't set a precise `Content-Type`
/// on the multipart field, so the backend receives `application/octet-stream`. Without
/// a correct content_type, the mobile client can't distinguish images from videos or
/// documents for in-app previews.
fn infer_content_type_from_ext(filename: &str) -> Option<&'static str> {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        // images
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "bmp" => Some("image/bmp"),
        "svg" => Some("image/svg+xml"),
        "heic" | "heif" => Some("image/heic"),
        // video
        "mp4" => Some("video/mp4"),
        "mov" => Some("video/quicktime"),
        "webm" => Some("video/webm"),
        "mkv" => Some("video/x-matroska"),
        "avi" => Some("video/x-msvideo"),
        "3gp" => Some("video/3gpp"),
        // audio
        "mp3" => Some("audio/mpeg"),
        "m4a" => Some("audio/mp4"),
        "wav" => Some("audio/wav"),
        "ogg" => Some("audio/ogg"),
        // docs
        "pdf" => Some("application/pdf"),
        "doc" => Some("application/msword"),
        "docx" => Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
        "xls" => Some("application/vnd.ms-excel"),
        "xlsx" => Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
        "ppt" => Some("application/vnd.ms-powerpoint"),
        "pptx" => Some("application/vnd.openxmlformats-officedocument.presentationml.presentation"),
        "txt" => Some("text/plain"),
        "csv" => Some("text/csv"),
        "json" => Some("application/json"),
        "xml" => Some("application/xml"),
        // archives
        "zip" => Some("application/zip"),
        "rar" => Some("application/x-rar-compressed"),
        "7z" => Some("application/x-7z-compressed"),
        _ => None,
    }
}

/// Resolve content_type preferring multipart field, falling back to extension inference.
/// Only falls back when the multipart-provided value is empty or the generic
/// `application/octet-stream` placeholder.
fn resolve_content_type(multipart_ct: Option<&str>, filename: &str) -> String {
    let trimmed = multipart_ct.map(str::trim).unwrap_or("");
    if !trimmed.is_empty() && trimmed != "application/octet-stream" {
        return trimmed.to_string();
    }
    infer_content_type_from_ext(filename)
        .unwrap_or("application/octet-stream")
        .to_string()
}

fn extract_auth_token(headers: &HeaderMap, query_token: Option<&str>) -> Result<String, Response> {
    if let Some(token) = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "))
        .map(|h| h[7..].to_string())
    {
        return Ok(token);
    }

    if let Some(t) = query_token {
        let trimmed = t.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    Err((StatusCode::UNAUTHORIZED, "Missing Token").into_response())
}

async fn authorize_file_access(
    state: &AppState,
    token: &str,
    file_id: &str,
) -> Result<(), Response> {
    let claims = state
        .auth_service
        .validate_token(token)
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid Token").into_response())?;

    let record = state
        .storage_service
        .get_file(file_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND.into_response())?;

    if claims.is_super_admin {
        return Ok(());
    }

    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| (StatusCode::FORBIDDEN, "No Tenant Context").into_response())?;
    let tenant_ok = tenant_id == record.tenant_id;
    if !tenant_ok {
        return Err((StatusCode::FORBIDDEN, "No Tenant Context").into_response());
    }

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "storage_files", "read")
        .await
        .map_err(|_| (StatusCode::FORBIDDEN, "Forbidden").into_response())?;

    // Non-admin users can only access files they uploaded (unless they have read_all)
    let has_read_all = state
        .auth_service
        .has_permission(&claims.sub, &tenant_id, "storage_files", "read_all")
        .await
        .unwrap_or(false);
    if !has_read_all && record.uploaded_by.as_deref() != Some(claims.sub.as_str()) {
        return Err((StatusCode::FORBIDDEN, "Forbidden").into_response());
    }

    Ok(())
}

#[derive(serde::Deserialize)]
pub struct ListFileParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub search: Option<String>,
}

#[derive(serde::Deserialize, Default)]
pub struct UploadFileQuery {
    pub payment_invoice_id: Option<String>,
    pub ticket_id: Option<String>,
    pub support_ticket_attachment: Option<bool>,
}

async fn can_upload_ticket_attachment(
    state: &AppState,
    claims: &crate::services::Claims,
    tenant_id: &str,
    ticket_id: &str,
) -> bool {
    let created_by: Result<Option<String>, _> = sqlx::query_scalar(
        r#"
        SELECT created_by
        FROM support_tickets
        WHERE id = $1 AND tenant_id = $2
        LIMIT 1
        "#,
    )
    .bind(ticket_id)
    .bind(tenant_id)
    .fetch_optional(&state.auth_service.pool)
    .await;

    match created_by {
        Ok(Some(owner)) if owner == claims.sub => true,
        Ok(Some(_)) => state
            .auth_service
            .has_permission(&claims.sub, tenant_id, "support", "read_all")
            .await
            .unwrap_or(false),
        Ok(None) => false,
        Err(e) => {
            warn!("[Upload] ticket attachment ownership check failed: {}", e);
            false
        }
    }
}

async fn can_upload_payment_proof(
    state: &AppState,
    claims: &crate::services::Claims,
    tenant_id: &str,
    invoice_id: &str,
) -> bool {
    let invoice = match state.payment_service.get_invoice(invoice_id).await {
        Ok(inv) => inv,
        Err(_) => return false,
    };

    if invoice.tenant_id != tenant_id {
        return false;
    }

    if state
        .auth_service
        .check_permission(&claims.sub, tenant_id, "billing", "manage")
        .await
        .is_ok()
    {
        return true;
    }

    // Allow portal customers to upload proof for their own invoices
    // (no specific permission needed beyond being a linked customer user)
    let customer_id = match state
        .customer_service
        .get_portal_customer_id(&claims.sub, tenant_id)
        .await
    {
        Ok(id) => id,
        Err(_) => return false,
    };

    state
        .payment_service
        .customer_owns_package_invoice(tenant_id, &customer_id, invoice_id)
        .await
        .unwrap_or(false)
}

pub async fn list_files(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ListFileParams>,
) -> Response {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "))
        .map(|h| h[7..].to_string());

    let token = match auth_header {
        Some(t) => t,
        None => return (StatusCode::UNAUTHORIZED, "Missing Token").into_response(),
    };

    let claims = match state.auth_service.validate_token(&token).await {
        Ok(c) => c,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid Token").into_response(),
    };

    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(20);

    info!(
        "[ListFiles] User: {}, Tenant: {:?}, SuperAdmin: {}",
        claims.sub, claims.tenant_id, claims.is_super_admin
    );

    if let Some(tid) = claims.tenant_id {
        if state
            .auth_service
            .check_permission(&claims.sub, &tid, "storage_console", "read")
            .await
            .is_err()
        {
            return (StatusCode::FORBIDDEN, "Forbidden").into_response();
        }

        info!("[ListFiles] Branch: Tenant Mode ({})", tid);
        match state
            .storage_service
            .list_tenant_files(&tid, page, per_page, params.search)
            .await
        {
            Ok((data, total)) => {
                info!("[ListFiles] Found {} files for tenant", total);
                Json(crate::models::PaginatedResponse {
                    data,
                    total,
                    page,
                    per_page,
                })
                .into_response()
            }
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    } else if claims.is_super_admin {
        info!("[ListFiles] Branch: Admin Mode (All Files)");
        match state
            .storage_service
            .list_all_files(page, per_page, params.search)
            .await
        {
            Ok((data, total)) => {
                info!("[ListFiles] Found {} files total", total);
                Json(crate::models::PaginatedResponse {
                    data,
                    total,
                    page,
                    per_page,
                })
                .into_response()
            }
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    } else {
        warn!("[ListFiles] Access Denied: No Tenant Context");
        (StatusCode::FORBIDDEN, "No Tenant Context").into_response()
    }
}

pub async fn delete_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
) -> Response {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "))
        .map(|h| h[7..].to_string());

    let token = match auth_header {
        Some(t) => t,
        None => return (StatusCode::UNAUTHORIZED, "Missing Token").into_response(),
    };

    let claims = match state.auth_service.validate_token(&token).await {
        Ok(c) => c,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid Token").into_response(),
    };
    let ip = extract_ip(&headers, addr);

    // Best-effort: fetch record for audit details
    let record = state.storage_service.get_file(&id).await.ok();

    if let Some(tid) = claims.tenant_id {
        if state
            .auth_service
            .check_permission(&claims.sub, &tid, "storage_files", "delete")
            .await
            .is_err()
        {
            return (StatusCode::FORBIDDEN, "Forbidden").into_response();
        }

        match state.storage_service.delete_tenant_file(&id, &tid).await {
            Ok(_) => {
                if let Some(r) = record.as_ref().filter(|r| r.tenant_id == tid) {
                    let details = serde_json::json!({
                        "file_id": r.id,
                        "tenant_id": r.tenant_id,
                        "original_name": r.original_name,
                        "size": r.size,
                        "storage_provider": r.storage_provider
                    })
                    .to_string();
                    state
                        .audit_service
                        .log(
                            Some(&claims.sub),
                            Some(&tid),
                            "delete",
                            "file_records",
                            Some(&id),
                            Some(details.as_str()),
                            Some(&ip),
                        )
                        .await;
                }
                StatusCode::OK.into_response()
            }
            Err(crate::error::AppError::NotFound(msg)) => {
                (StatusCode::NOT_FOUND, msg).into_response()
            }
            Err(crate::error::AppError::Forbidden(msg)) => {
                (StatusCode::FORBIDDEN, msg).into_response()
            }
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    } else if claims.is_super_admin {
        match state.storage_service.delete_file(&id).await {
            Ok(_) => {
                if let Some(r) = record.as_ref() {
                    let details = serde_json::json!({
                        "file_id": r.id,
                        "tenant_id": r.tenant_id,
                        "original_name": r.original_name,
                        "size": r.size,
                        "storage_provider": r.storage_provider
                    })
                    .to_string();
                    state
                        .audit_service
                        .log(
                            Some(&claims.sub),
                            None,
                            "delete",
                            "file_records",
                            Some(&id),
                            Some(details.as_str()),
                            Some(&ip),
                        )
                        .await;
                }
                StatusCode::OK.into_response()
            }
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    } else {
        (StatusCode::FORBIDDEN, "No Tenant Context").into_response()
    }
}

pub async fn serve_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<FileAccessQuery>,
) -> Response {
    let token = match extract_auth_token(&headers, q.token.as_deref()) {
        Ok(t) => t,
        Err(resp) => return resp,
    };

    if let Err(resp) = authorize_file_access(&state, &token, &id).await {
        return resp;
    }

    let (record, content) = match state.storage_service.get_file_content(&id).await {
        Ok(res) => res,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    match content {
        StorageContent::Local(path) => {
            let file_size = match fs::metadata(&path).await {
                Ok(m) => m.len(),
                Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };

            let range_header = headers.get(header::RANGE).and_then(|v| v.to_str().ok());

            if let Some(range_str) = range_header {
                if let Some(range) = parse_range(range_str, file_size) {
                    let start = range.0;
                    let end = range.1;
                    let chunk_size = (end - start) + 1;

                    let mut file = match File::open(path).await {
                        Ok(f) => f,
                        Err(_) => return StatusCode::NOT_FOUND.into_response(),
                    };

                    if (file.seek(std::io::SeekFrom::Start(start)).await).is_err() {
                        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                    }

                    let stream = ReaderStream::new(file.take(chunk_size));
                    let body = Body::from_stream(stream);

                    return Response::builder()
                        .status(StatusCode::PARTIAL_CONTENT)
                        .header(header::CONTENT_TYPE, &record.content_type)
                        .header(header::ACCEPT_RANGES, "bytes")
                        .header(
                            header::CONTENT_RANGE,
                            format!("bytes {}-{}/{}", start, end, file_size),
                        )
                        .header(header::CONTENT_LENGTH, chunk_size)
                        .header(
                            header::CONTENT_DISPOSITION,
                            format!("inline; filename=\"{}\"", record.original_name),
                        )
                        .body(body)
                        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
                }
            }

            let file = match File::open(path).await {
                Ok(f) => f,
                Err(_) => return StatusCode::NOT_FOUND.into_response(),
            };
            let stream = ReaderStream::new(file);
            let body = Body::from_stream(stream);

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, record.content_type)
                .header(header::ACCEPT_RANGES, "bytes")
                .header(header::CONTENT_LENGTH, file_size)
                .header(
                    header::CONTENT_DISPOSITION,
                    format!("inline; filename=\"{}\"", record.original_name),
                )
                .body(body)
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
        StorageContent::S3(byte_stream) => {
            let body = Body::from_stream(ReaderStream::new(byte_stream.into_async_read()));
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, record.content_type)
                .header(
                    header::CONTENT_DISPOSITION,
                    format!("inline; filename=\"{}\"", record.original_name),
                )
                .body(body)
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

pub async fn download_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<FileAccessQuery>,
) -> Response {
    let token = match extract_auth_token(&headers, q.token.as_deref()) {
        Ok(t) => t,
        Err(resp) => return resp,
    };

    if let Err(resp) = authorize_file_access(&state, &token, &id).await {
        return resp;
    }

    let (record, content) = match state.storage_service.get_file_content(&id).await {
        Ok(res) => res,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    let body = match content {
        StorageContent::Local(path) => {
            let file = match File::open(path).await {
                Ok(f) => f,
                Err(_) => return StatusCode::NOT_FOUND.into_response(),
            };
            Body::from_stream(ReaderStream::new(file))
        }
        StorageContent::S3(byte_stream) => {
            Body::from_stream(ReaderStream::new(byte_stream.into_async_read()))
        }
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, record.content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", record.original_name),
        )
        .body(body)
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn parse_range(range_str: &str, file_size: u64) -> Option<(u64, u64)> {
    if !range_str.starts_with("bytes=") {
        return None;
    }

    let range = &range_str[6..];
    let parts: Vec<&str> = range.split('-').collect();

    if parts.len() != 2 {
        return None;
    }

    let start = parts[0].parse::<u64>().ok().unwrap_or(0);
    let end = parts[1].parse::<u64>().ok().unwrap_or(file_size - 1);

    let end = if end >= file_size { file_size - 1 } else { end };

    if start > end {
        return None;
    }

    Some((start, end))
}

pub async fn upload_file_http(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<UploadFileQuery>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut multipart: Multipart,
) -> Response {
    info!("[Upload] 📥 New upload request received");

    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "))
        .map(|h| h[7..].to_string());

    let token = match auth_header {
        Some(t) => t,
        None => {
            warn!("[Upload] 🚫 Missing authorization header");
            return (StatusCode::UNAUTHORIZED, "Missing Token").into_response();
        }
    };

    let claims = match state.auth_service.validate_token(&token).await {
        Ok(c) => c,
        Err(e) => {
            warn!("[Upload] 🚫 Invalid token: {}", e);
            return (StatusCode::UNAUTHORIZED, "Invalid Token").into_response();
        }
    };
    let ip = extract_ip(&headers, addr);

    let tenant_id = match claims.tenant_id.clone() {
        Some(tid) => tid,
        None => {
            if claims.is_super_admin {
                "system".to_string()
            } else {
                return (StatusCode::FORBIDDEN, "No Tenant Context").into_response();
            }
        }
    };

    if !claims.is_super_admin {
        let has_storage_upload = state
            .auth_service
            .check_permission(&claims.sub, &tenant_id, "storage_files", "upload")
            .await
            .is_ok();

        if !has_storage_upload {
            let ticket_id = query
                .ticket_id
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty());
            let is_ticket_attachment = query.support_ticket_attachment.unwrap_or(false);

            if is_ticket_attachment {
                let Some(ticket_id) = ticket_id else {
                    return (StatusCode::FORBIDDEN, "Forbidden").into_response();
                };

                if !can_upload_ticket_attachment(&state, &claims, &tenant_id, ticket_id).await {
                    return (StatusCode::FORBIDDEN, "Forbidden").into_response();
                }
            } else {
                let payment_invoice_id = query
                    .payment_invoice_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty());

                let Some(invoice_id) = payment_invoice_id else {
                    return (StatusCode::FORBIDDEN, "Forbidden").into_response();
                };

                if !can_upload_payment_proof(&state, &claims, &tenant_id, invoice_id).await {
                    return (StatusCode::FORBIDDEN, "Forbidden").into_response();
                }
            }
        }
    }

    let max_mb: u64 = state
        .settings_service
        .get_value(None, "storage_max_file_size_mb")
        .await
        .unwrap_or(None)
        .and_then(|v| v.parse().ok())
        .unwrap_or(500);
    let max_bytes = max_mb * 1024 * 1024;

    let allowed_exts_str = state
        .settings_service
        .get_value(None, "storage_allowed_extensions")
        .await
        .unwrap_or(None)
        .unwrap_or_else(|| "jpg,jpeg,png,gif,pdf,doc,docx,xls,xlsx,zip,mp4,mov".to_string());
    let allowed_exts: Vec<String> = allowed_exts_str
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .collect();

    info!(
        "[Upload] 👤 User: {}, Tenant: {}, Limit: {}MB",
        claims.sub, tenant_id, max_mb
    );

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            let file_name = field.file_name().unwrap_or("upload.bin").to_string();
            let content_type =
                resolve_content_type(field.content_type(), &file_name);

            let ext = std::path::Path::new(&file_name)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();

            if !allowed_exts.contains(&ext) && !allowed_exts.contains(&"*".to_string()) {
                warn!("[Upload] ❌ Blocked extension: .{}\n", ext);
                return (
                    StatusCode::BAD_REQUEST,
                    format!("File type '.{}' not allowed", ext),
                )
                    .into_response();
            }

            let (path, safe_name, file_id) = match state
                .storage_service
                .prepare_upload_path(&tenant_id, &file_name)
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    error!("[Upload] ❌ Path preparation failed: {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
                }
            };

            info!("[Upload] 📝 Streaming to disk: {:?}", path);

            let mut file = match File::create(&path).await {
                Ok(f) => f,
                Err(e) => {
                    error!("[Upload] ❌ File creation failed: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to create file: {}", e),
                    )
                        .into_response();
                }
            };

            let mut current_size: u64 = 0;
            let mut last_reported_mb = 0;
            let mut stream = field;

            while let Ok(Some(chunk)) = stream.chunk().await {
                let chunk_len = chunk.len() as u64;
                current_size += chunk_len;

                let current_mb = current_size / (1024 * 1024);
                if current_mb >= last_reported_mb + 5 {
                    info!("[Upload] ⏳ Progress: {} MB received...", current_mb);
                    last_reported_mb = current_mb;
                }

                if current_size > max_bytes {
                    warn!(
                        "[Upload] ⚠️ File too large ({}MB > {}MB). Aborting.",
                        current_mb, max_mb
                    );
                    drop(file);
                    let _ = tokio::fs::remove_file(&path).await;
                    return (
                        StatusCode::PAYLOAD_TOO_LARGE,
                        format!("File exceeds limit of {} MB", max_mb),
                    )
                        .into_response();
                }

                if let Err(e) = file.write_all(&chunk).await {
                    error!("[Upload] ❌ Write error at {}MB: {}", current_mb, e);
                    drop(file);
                    let _ = tokio::fs::remove_file(&path).await;
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Disk write error").into_response();
                }
            }

            let _ = file.flush().await;
            info!(
                "[Upload] ✅ Write finished. Total size: {} MB. Registering...",
                current_size / (1024 * 1024)
            );

            let result = state
                .storage_service
                .register_upload(
                    &tenant_id,
                    &file_id,
                    &file_name,
                    &safe_name,
                    &path.to_string_lossy(),
                    &content_type,
                    current_size as i64,
                    "local", // Direct HTTP upload defaults to local for now
                    Some(&claims.sub),
                    false, // Count quota
                )
                .await;

            return match result {
                Ok(record) => {
                    info!("[Upload] ✨ Success! ID: {}", record.id);

                    let details = serde_json::json!({
                        "file_id": record.id,
                        "tenant_id": record.tenant_id,
                        "original_name": record.original_name,
                        "size": record.size,
                        "storage_provider": record.storage_provider,
                    })
                    .to_string();
                    state
                        .audit_service
                        .log(
                            Some(&claims.sub),
                            Some(&tenant_id),
                            "create",
                            "file_records",
                            Some(&record.id),
                            Some(details.as_str()),
                            Some(&ip),
                        )
                        .await;

                    Json(record).into_response()
                }
                Err(e) => {
                    error!("[Upload] ❌ Database registration failed: {}", e);
                    let _ = tokio::fs::remove_file(&path).await;
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
                }
            };
        }
    }

    warn!("[Upload] ⚠️ No file field found");
    (StatusCode::BAD_REQUEST, "No file field found").into_response()
}

#[derive(serde::Serialize)]
pub struct InitResponse {
    pub upload_id: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompleteRequest {
    pub upload_id: String,
    pub file_name: String,
    pub content_type: String,
}

pub async fn init_upload(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "))
        .map(|h| h[7..].to_string());

    if let Some(token) = auth_header {
        let claims = match state.auth_service.validate_token(&token).await {
            Ok(c) => c,
            Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid Token").into_response(),
        };

        if !claims.is_super_admin {
            let Some(tid) = claims.tenant_id else {
                return (StatusCode::FORBIDDEN, "No Tenant Context").into_response();
            };
            if state
                .auth_service
                .check_permission(&claims.sub, &tid, "storage_files", "upload")
                .await
                .is_err()
            {
                return (StatusCode::FORBIDDEN, "Forbidden").into_response();
            }
        }
    } else {
        return (StatusCode::UNAUTHORIZED, "Missing Token").into_response();
    }

    match state.storage_service.init_chunk_session().await {
        Ok(id) => Json(InitResponse { upload_id: id }).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn upload_chunk(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Response {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "))
        .map(|h| h[7..].to_string());

    if let Some(token) = auth_header {
        let claims = match state.auth_service.validate_token(&token).await {
            Ok(c) => c,
            Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid Token").into_response(),
        };
        if !claims.is_super_admin {
            let Some(tid) = claims.tenant_id else {
                return (StatusCode::FORBIDDEN, "No Tenant Context").into_response();
            };
            if state
                .auth_service
                .check_permission(&claims.sub, &tid, "storage_files", "upload")
                .await
                .is_err()
            {
                return (StatusCode::FORBIDDEN, "Forbidden").into_response();
            }
        }
    } else {
        return (StatusCode::UNAUTHORIZED, "Missing Token").into_response();
    }

    let mut upload_id = String::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        if name == "upload_id" {
            if let Ok(txt) = field.text().await {
                upload_id = txt;
            }
        } else if name == "chunk" {
            if upload_id.is_empty() {
                return (StatusCode::BAD_REQUEST, "upload_id must come before chunk")
                    .into_response();
            }

            let data = match field.bytes().await {
                Ok(b) => b,
                Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
            };

            match state.storage_service.process_chunk(&upload_id, &data).await {
                Ok(_) => return StatusCode::OK.into_response(),
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
                }
            }
        }
    }

    (StatusCode::BAD_REQUEST, "Missing chunk data").into_response()
}

pub async fn complete_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<CompleteRequest>,
) -> Response {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "))
        .map(|h| h[7..].to_string());

    let token = match auth_header {
        Some(t) => t,
        None => return (StatusCode::UNAUTHORIZED, "Missing Token").into_response(),
    };

    let claims = match state.auth_service.validate_token(&token).await {
        Ok(c) => c,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid Token").into_response(),
    };
    let ip = extract_ip(&headers, addr);

    let tenant_id = match claims.tenant_id {
        Some(tid) => tid,
        None => {
            if claims.is_super_admin {
                "system".to_string()
            } else {
                return StatusCode::FORBIDDEN.into_response();
            }
        }
    };

    if !claims.is_super_admin {
        if state
            .auth_service
            .check_permission(&claims.sub, &tenant_id, "storage_files", "upload")
            .await
            .is_err()
        {
            return (StatusCode::FORBIDDEN, "Forbidden").into_response();
        }
    }

    match state
        .storage_service
        .complete_chunk_session(
            &tenant_id,
            &payload.upload_id,
            &payload.file_name,
            &payload.content_type,
            Some(&claims.sub),
        )
        .await
    {
        Ok(record) => {
            let details = serde_json::json!({
                "file_id": record.id,
                "tenant_id": record.tenant_id,
                "original_name": record.original_name,
                "size": record.size,
                "storage_provider": record.storage_provider,
                "upload_id": payload.upload_id,
            })
            .to_string();
            state
                .audit_service
                .log(
                    Some(&claims.sub),
                    Some(&tenant_id),
                    "create",
                    "file_records",
                    Some(&record.id),
                    Some(details.as_str()),
                    Some(&ip),
                )
                .await;

            Json(record).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Serve a file that is a ticket attachment.
/// Authorization: user must be the ticket creator OR have support:read_all permission.
/// This bypasses storage_files:read so customers can see attachments in their tickets.
pub async fn serve_ticket_attachment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(file_id): Path<String>,
    Query(q): Query<FileAccessQuery>,
) -> Response {
    let token = match extract_auth_token(&headers, q.token.as_deref()) {
        Ok(t) => t,
        Err(resp) => return resp,
    };

    let claims = match state.auth_service.validate_token(&token).await {
        Ok(c) => c,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid Token").into_response(),
    };

    let tenant_id = match claims.tenant_id.clone() {
        Some(tid) => tid,
        None => return (StatusCode::FORBIDDEN, "No Tenant Context").into_response(),
    };

    // Check if this file is linked to a support ticket via support_ticket_attachments.
    // Also fetch the ticket creator so we can verify access.
    #[derive(sqlx::FromRow)]
    struct TicketLink {
        ticket_id: String,
        created_by: Option<String>,
    }

    let link = sqlx::query_as::<_, TicketLink>(
        r#"
        SELECT t.id AS ticket_id, t.created_by
        FROM support_ticket_attachments a
        JOIN support_ticket_messages m ON m.id = a.message_id
        JOIN support_tickets t ON t.id = m.ticket_id
        WHERE a.file_id = $1 AND t.tenant_id = $2
        LIMIT 1
        "#,
    )
    .bind(&file_id)
    .bind(&tenant_id)
    .fetch_optional(&state.auth_service.pool)
    .await;

    let link = match link {
        Ok(Some(l)) => l,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, "Attachment not found").into_response();
        }
        Err(_) => {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // Access: ticket creator OR staff with support:read_all
    let _ = link.ticket_id; // used by SQL join, not needed after fetch
    let is_creator = link.created_by.as_deref() == Some(claims.sub.as_str());
    let is_staff = state
        .auth_service
        .has_permission(&claims.sub, &tenant_id, "support", "read_all")
        .await
        .unwrap_or(false);

    if !is_creator && !is_staff {
        return (StatusCode::FORBIDDEN, "Forbidden").into_response();
    }

    // Serve the file content (reuse the same logic as serve_file)
    let (record, content) = match state.storage_service.get_file_content(&file_id).await {
        Ok(res) => res,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    match content {
        crate::services::storage_service::StorageContent::Local(path) => {
            let file_size = match tokio::fs::metadata(&path).await {
                Ok(m) => m.len(),
                Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };

            let file = match tokio::fs::File::open(path).await {
                Ok(f) => f,
                Err(_) => return StatusCode::NOT_FOUND.into_response(),
            };
            let stream = tokio_util::io::ReaderStream::new(file);
            let body = Body::from_stream(stream);

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, &record.content_type)
                .header(header::ACCEPT_RANGES, "bytes")
                .header(header::CONTENT_LENGTH, file_size)
                .header(
                    header::CONTENT_DISPOSITION,
                    format!("inline; filename=\"{}\"", record.original_name),
                )
                .body(body)
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
        crate::services::storage_service::StorageContent::S3(byte_stream) => {
            let body = Body::from_stream(ReaderStream::new(byte_stream.into_async_read()));
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, &record.content_type)
                .header(
                    header::CONTENT_DISPOSITION,
                    format!("inline; filename=\"{}\"", record.original_name),
                )
                .body(body)
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}
