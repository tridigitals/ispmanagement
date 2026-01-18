use axum::{
    extract::{Path, State, Multipart, Query},
    http::{header, StatusCode, HeaderMap},
    response::{IntoResponse, Response},
    body::{Body, Bytes},
    Json,
};
use tokio_util::io::ReaderStream;
use tokio::fs;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, AsyncReadExt, AsyncSeekExt}; // Added imports
use crate::http::AppState;
use tracing::{info, warn, error};

#[derive(serde::Deserialize)]
pub struct ListFileParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub search: Option<String>,
}

pub async fn list_files(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ListFileParams>,
) -> Response {
    let auth_header = headers.get(header::AUTHORIZATION)
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

    info!("[ListFiles] User: {}, Tenant: {:?}, SuperAdmin: {}", claims.sub, claims.tenant_id, claims.is_super_admin);

    if let Some(tid) = claims.tenant_id {
        info!("[ListFiles] Branch: Tenant Mode ({})", tid);
        match state.storage_service.list_tenant_files(&tid, page, per_page, params.search).await {
            Ok((data, total)) => {
                info!("[ListFiles] Found {} files for tenant", total);
                Json(crate::models::PaginatedResponse { data, total, page, per_page }).into_response()
            },
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    } else if claims.is_super_admin {
        info!("[ListFiles] Branch: Admin Mode (All Files)");
        match state.storage_service.list_all_files(page, per_page, params.search).await {
            Ok((data, total)) => {
                info!("[ListFiles] Found {} files total", total);
                Json(crate::models::PaginatedResponse { data, total, page, per_page }).into_response()
            },
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
    Path(id): Path<String>,
) -> Response {
    let auth_header = headers.get(header::AUTHORIZATION)
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

    if let Some(tid) = claims.tenant_id {
        match state.storage_service.delete_tenant_file(&id, &tid).await {
            Ok(_) => StatusCode::OK.into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    } else if claims.is_super_admin {
        match state.storage_service.delete_file(&id).await {
            Ok(_) => StatusCode::OK.into_response(),
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
) -> Response {
    // ... existing serve_file logic ...
    // 1. Get file record from DB
    let record = match state.storage_service.get_file(&id).await {
        Ok(r) => r,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    let path = std::path::Path::new(&record.path);
    if !path.exists() {
        return StatusCode::NOT_FOUND.into_response();
    }

    // 2. Handle Range Header for Video Seeking
    let file_size = match fs::metadata(path).await {
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

            // Seek to start position
            if let Err(_) = file.seek(std::io::SeekFrom::Start(start)).await {
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }

            // Stream only the requested part
            let stream = ReaderStream::new(file.take(chunk_size));
            let body = Body::from_stream(stream);

            return Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::CONTENT_TYPE, &record.content_type)
                .header(header::ACCEPT_RANGES, "bytes")
                .header(header::CONTENT_RANGE, format!("bytes {}-{}/{}", start, end, file_size))
                .header(header::CONTENT_LENGTH, chunk_size)
                .header(header::CONTENT_DISPOSITION, format!("inline; filename=\"{}\"", record.original_name))
                .body(body)
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    }

    // 3. Fallback: Serve full file (200 OK)
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
        .header(header::CONTENT_DISPOSITION, format!("inline; filename=\"{}\"", record.original_name))
        .body(body)
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

/// Force download file (Attachment)
pub async fn download_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    // 1. Get file record from DB
    let record = match state.storage_service.get_file(&id).await {
        Ok(r) => r,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    let path = std::path::Path::new(&record.path);
    if !path.exists() {
        return StatusCode::NOT_FOUND.into_response();
    }

    // 2. Open file
    let file = match File::open(path).await {
        Ok(f) => f,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    // 3. Send with Attachment disposition
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, record.content_type) // Or application/octet-stream to force?
        // attachment forces "Save As" dialog
        .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", record.original_name))
        .body(body)
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

/// Helper to parse Range header: "bytes=start-end"
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

    // Clamp end to file size
    let end = if end >= file_size { file_size - 1 } else { end };

    if start > end {
        return None;
    }

    Some((start, end))
}

pub async fn upload_file_http(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Response {
    info!("[Upload] 📥 New upload request received");

    // 1. Validate Token
    let auth_header = headers.get(header::AUTHORIZATION)
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

    let tenant_id = match claims.tenant_id {
        Some(tid) => tid,
        None => {
            if claims.is_super_admin {
                "system".to_string()
            } else {
                return (StatusCode::FORBIDDEN, "No Tenant Context").into_response()
            }
        }
    };

    // 2. Get Global Limits
    let max_mb: u64 = state.settings_service.get_value(None, "storage_max_file_size_mb").await
        .unwrap_or(None)
        .and_then(|v| v.parse().ok())
        .unwrap_or(500);
    let max_bytes = max_mb * 1024 * 1024;

    let allowed_exts_str = state.settings_service.get_value(None, "storage_allowed_extensions").await
        .unwrap_or(None)
        .unwrap_or_else(|| "jpg,jpeg,png,gif,pdf,doc,docx,xls,xlsx,zip,mp4,mov".to_string());
    let allowed_exts: Vec<String> = allowed_exts_str.split(',').map(|s| s.trim().to_lowercase()).collect();

    info!("[Upload] 👤 User: {}, Tenant: {}, Limit: {}MB", claims.sub, tenant_id, max_mb);

    // 3. Process Stream
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        
        if name == "file" {
            let file_name = field.file_name().unwrap_or("upload.bin").to_string();
            let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
            
            // Check Extension
            let ext = std::path::Path::new(&file_name)
                .extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            
            if !allowed_exts.contains(&ext) && !allowed_exts.contains(&"*".to_string()) {
                 warn!("[Upload] ❌ Blocked extension: .{}", ext);
                 return (StatusCode::BAD_REQUEST, format!("File type '.{}' not allowed", ext)).into_response();
            }

            // Prepare Path
            let (path, safe_name, file_id) = match state.storage_service.prepare_upload_path(&tenant_id, &file_name).await {
                Ok(p) => p,
                Err(e) => {
                    error!("[Upload] ❌ Path preparation failed: {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
                }
            };

            info!("[Upload] 📝 Streaming to disk: {:?}", path);

            // Stream Write
            let mut file = match File::create(&path).await {
                Ok(f) => f,
                Err(e) => {
                    error!("[Upload] ❌ File creation failed: {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create file: {}", e)).into_response();
                }
            };

            let mut current_size: u64 = 0;
            let mut last_reported_mb = 0;
            let mut stream = field;

            while let Ok(Some(chunk)) = stream.chunk().await {
                let chunk_len = chunk.len() as u64;
                current_size += chunk_len;
                
                // Log progress every 5MB
                let current_mb = current_size / (1024 * 1024);
                if current_mb >= last_reported_mb + 5 {
                    info!("[Upload] ⏳ Progress: {} MB received...", current_mb);
                    last_reported_mb = current_mb;
                }

                if current_size > max_bytes {
                    warn!("[Upload] ⚠️ File too large ({}MB > {}MB). Aborting.", current_mb, max_mb);
                    drop(file);
                    let _ = tokio::fs::remove_file(&path).await;
                    return (StatusCode::PAYLOAD_TOO_LARGE, format!("File exceeds limit of {} MB", max_mb)).into_response();
                }

                if let Err(e) = file.write_all(&chunk).await {
                    error!("[Upload] ❌ Write error at {}MB: {}", current_mb, e);
                    drop(file);
                    let _ = tokio::fs::remove_file(&path).await;
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Disk write error").into_response();
                }
            }

            // Important: sync data to disk
            let _ = file.flush().await;
            info!("[Upload] ✅ Write finished. Total size: {} MB. Registering...", current_size / (1024 * 1024));

            // Register in DB
            let result = state.storage_service.register_upload(
                &tenant_id,
                &file_id,
                &file_name,
                &safe_name,
                &path.to_string_lossy(),
                &content_type,
                current_size as i64,
                Some(&claims.sub)
            ).await;

            return match result {
                Ok(record) => {
                    info!("[Upload] ✨ Success! ID: {}", record.id);
                    Json(record).into_response()
                },
                Err(e) => {
                    error!("[Upload] ❌ Database registration failed: {}", e);
                    let _ = tokio::fs::remove_file(&path).await;
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
                },
            };
        }
    }

    warn!("[Upload] ⚠️ No file field found");
    (StatusCode::BAD_REQUEST, "No file field found").into_response()
}

// --- Chunked Upload Handlers ---

#[derive(serde::Serialize)]
pub struct InitResponse {
    pub upload_id: String,
}

#[derive(serde::Deserialize)]
pub struct CompleteRequest {
    pub upload_id: String,
    pub file_name: String,
    pub content_type: String,
}

pub async fn init_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    // Auth Check
    let auth_header = headers.get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "))
        .map(|h| h[7..].to_string());

    if let Some(token) = auth_header {
        if state.auth_service.validate_token(&token).await.is_err() {
            return (StatusCode::UNAUTHORIZED, "Invalid Token").into_response();
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
    // Auth Check (Quick check, session is validated by existence of temp file mostly)
    let auth_header = headers.get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "))
        .map(|h| h[7..].to_string());

    if let Some(token) = auth_header {
        if state.auth_service.validate_token(&token).await.is_err() {
            return (StatusCode::UNAUTHORIZED, "Invalid Token").into_response();
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
                return (StatusCode::BAD_REQUEST, "upload_id must come before chunk").into_response();
            }

            let data = match field.bytes().await {
                Ok(b) => b,
                Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
            };

            match state.storage_service.process_chunk(&upload_id, &data).await {
                Ok(_) => return StatusCode::OK.into_response(),
                Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
            }
        }
    }

    (StatusCode::BAD_REQUEST, "Missing chunk data").into_response()
}

pub async fn complete_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CompleteRequest>,
) -> Response {
    // Auth Check & Tenant ID
    let auth_header = headers.get(header::AUTHORIZATION)
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

    let tenant_id = match claims.tenant_id {
        Some(tid) => tid,
        None => if claims.is_super_admin { "system".to_string() } else { return StatusCode::FORBIDDEN.into_response() }
    };

    // Limits Check (Optional: Check final size vs limit here again)
    // For now, relying on service logic

    match state.storage_service.complete_chunk_session(
        &tenant_id, 
        &payload.upload_id, 
        &payload.file_name, 
        &payload.content_type, 
        Some(&claims.sub)
    ).await {
        Ok(record) => Json(record).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
