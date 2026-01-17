use axum::{
    extract::{Path, State, Multipart},
    http::{header, StatusCode, HeaderMap},
    response::{IntoResponse, Response},
    body::Body,
    Json,
};
use tokio_util::io::ReaderStream;
use tokio::fs::File;
use tokio::io::AsyncWriteExt; // Import for write_all
use crate::http::AppState;
use tracing::{info, warn, error};

pub async fn serve_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    // 1. Get file record from DB
    let record = match state.storage_service.get_file(&id).await {
        Ok(r) => r,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    // 2. Open file from disk
    let file = match File::open(&record.path).await {
        Ok(f) => f,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    // 3. Create stream
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    // 4. Set headers
    let headers = [
        (header::CONTENT_TYPE, record.content_type),
        (header::CONTENT_DISPOSITION, format!("inline; filename=\"{}\"", record.original_name)),
    ];

    (headers, body).into_response()
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
