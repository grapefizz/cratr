use actix_files as fs;
use actix_multipart::Multipart;
use actix_web::{
    get, middleware::Logger, post, web, App, HttpResponse, HttpServer, Result as ActixResult,
    cookie::Key,
};
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_identity::IdentityMiddleware;
#[cfg(feature = "server")]
use futures_util::TryStreamExt as _;
use serde::Serialize;
use std::fs::create_dir_all;
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;
use cratr::{FileInfo, StorageInfo, LoginRequest, LoginResponse, AuthStatus};
use clap::Parser;

const UPLOAD_DIR: &str = "./uploads";
const MAX_FILE_SIZE: usize = 16384 * 1024 * 1024; // 16384 MB
const MAX_FILE_COUNT: usize = 10;
const MAX_STORAGE_SIZE: u64 = 1024 * 1024 * 1024 * 1024; // 1024 GB total storage limit

// Default credentials - change these in production!
const DEFAULT_USERNAME: &str = "admin";
const DEFAULT_PASSWORD: &str = "admin";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable debug mode
    #[arg(long)]
    debug: bool,
}

#[derive(Clone)]
struct AppState {
    debug_mode: bool,
}

#[derive(Serialize)]
struct UploadResponse {
    success: bool,
    message: String,
    files: Vec<FileInfo>,
}

#[derive(Serialize)]
struct FileListResponse {
    files: Vec<FileInfo>,
}

#[derive(Serialize)]
struct DebugInfo {
    debug_mode: bool,
}

// Helper function to check if user is authenticated
fn is_authenticated(session: &actix_session::Session) -> bool {
    session.get::<String>("username").unwrap_or(None).is_some()
}

// Login endpoint
#[post("/login")]
async fn login(
    request: web::Json<LoginRequest>,
    session: actix_session::Session,
) -> ActixResult<HttpResponse> {
    // Simple credential check (in production, use proper password hashing)
    if request.username == DEFAULT_USERNAME && request.password == DEFAULT_PASSWORD {
        // Store user in session
        session.insert("username", &request.username)
            .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to create session: {}", e)))?;
            
        Ok(HttpResponse::Ok().json(LoginResponse {
            success: true,
            message: "Login successful".to_string(),
            authenticated: true,
        }))
    } else {
        Ok(HttpResponse::Unauthorized().json(LoginResponse {
            success: false,
            message: "Invalid credentials".to_string(),
            authenticated: false,
        }))
    }
}

// Logout endpoint
#[post("/logout")]
async fn logout(session: actix_session::Session) -> ActixResult<HttpResponse> {
    session.clear();
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Logged out successfully",
        "authenticated": false
    })))
}

// Check authentication status
#[get("/auth/status")]
async fn auth_status(session: actix_session::Session) -> ActixResult<HttpResponse> {
    let username = session.get::<String>("username").unwrap_or(None);
    let authenticated = username.is_some();
    
    Ok(HttpResponse::Ok().json(AuthStatus {
        authenticated,
        username,
    }))
}

// Authentication middleware wrapper
fn require_auth(session: &actix_session::Session) -> ActixResult<()> {
    if is_authenticated(session) {
        Ok(())
    } else {
        Err(actix_web::error::ErrorUnauthorized("Authentication required"))
    }
}

// Get storage information
#[get("/storage")]
async fn get_storage_info(session: actix_session::Session) -> ActixResult<HttpResponse> {
    require_auth(&session)?;
    let mut total_size = 0u64;
    let mut file_count = 0usize;

    if let Ok(entries) = std::fs::read_dir(UPLOAD_DIR) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    total_size += metadata.len();
                    file_count += 1;
                }
            }
        }
    }

    // Get disk space information
    let (disk_free, disk_total) = get_disk_space(UPLOAD_DIR);
    let disk_used = disk_total.saturating_sub(disk_free);
    let disk_used_percentage = if disk_total > 0 {
        (disk_used as f64 / disk_total as f64) * 100.0
    } else {
        0.0
    };

    let percentage = (total_size as f64 / MAX_STORAGE_SIZE as f64) * 100.0;
    let formatted_used = format_bytes(total_size);
    let formatted_disk_free = format_bytes(disk_free);
    let formatted_disk_total = format_bytes(disk_total);

    Ok(HttpResponse::Ok().json(StorageInfo {
        used_bytes: total_size,
        total_files: file_count,
        used_percentage: percentage,
        formatted_used,
        max_size_mb: MAX_STORAGE_SIZE / 1024 / 1024,
        disk_free_bytes: disk_free,
        disk_total_bytes: disk_total,
        disk_used_percentage,
        formatted_disk_free,
        formatted_disk_total,
    }))
}

// Serve the main HTML page
#[get("/")]
async fn index() -> ActixResult<HttpResponse> {
    let html = include_str!("../static/index.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// Get debug configuration
#[get("/debug")]
async fn get_debug_info(data: web::Data<AppState>) -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(DebugInfo {
        debug_mode: data.debug_mode,
    }))
}

// Handle file uploads
#[post("/upload")]
async fn upload_files(mut payload: Multipart, session: actix_session::Session) -> ActixResult<HttpResponse> {
    require_auth(&session)?;
    // Ensure upload directory exists
    create_dir_all(UPLOAD_DIR).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to create upload directory: {}", e))
    })?;

    let mut uploaded_files = Vec::new();
    let mut file_count = 0;

    while let Some(mut field) = payload.try_next().await? {
        let content_disposition = field.content_disposition();
        
        if let Some(filename) = content_disposition.and_then(|cd| cd.get_filename()) {
            if file_count >= MAX_FILE_COUNT {
                return Ok(HttpResponse::BadRequest().json(UploadResponse {
                    success: false,
                    message: format!("Maximum {} files allowed", MAX_FILE_COUNT),
                    files: vec![],
                }));
            }

            // Sanitize filename and add UUID to prevent conflicts
            let sanitized_filename = sanitize_filename(filename);
            let unique_filename = format!("{}_{}", Uuid::new_v4(), sanitized_filename);
            let filepath = PathBuf::from(UPLOAD_DIR).join(&unique_filename);
            let filepath_clone = filepath.clone();

            // Create the file
            let mut f = web::block(move || std::fs::File::create(filepath))
                .await?
                .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to create file: {}", e)))?;

            let mut file_size = 0;

            // Write file chunks
            while let Some(chunk) = field.try_next().await? {
                file_size += chunk.len();
                if file_size > MAX_FILE_SIZE {
                    // Remove the partially written file
                    let _ = std::fs::remove_file(&filepath_clone);
                    return Ok(HttpResponse::BadRequest().json(UploadResponse {
                        success: false,
                        message: format!("File too large. Maximum size is {} MB", MAX_FILE_SIZE / 1024 / 1024),
                        files: vec![],
                    }));
                }

                f = web::block(move || f.write_all(&chunk).map(|_| f))
                    .await?
                    .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to write file: {}", e)))?;
            }

            uploaded_files.push(FileInfo {
                name: sanitized_filename.clone(),
                size: file_size as u64,
                path: unique_filename.clone(),
                file_type: get_file_type_and_preview(&sanitized_filename).0,
                can_preview: get_file_type_and_preview(&sanitized_filename).1,
            });

            file_count += 1;
        }
    }

    if uploaded_files.is_empty() {
        Ok(HttpResponse::BadRequest().json(UploadResponse {
            success: false,
            message: "No files were uploaded".to_string(),
            files: vec![],
        }))
    } else {
        Ok(HttpResponse::Ok().json(UploadResponse {
            success: true,
            message: format!("Successfully uploaded {} file(s)", uploaded_files.len()),
            files: uploaded_files,
        }))
    }
}

// List all uploaded files
#[get("/files")]
async fn list_files(session: actix_session::Session) -> ActixResult<HttpResponse> {
    require_auth(&session)?;
    let mut files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(UPLOAD_DIR) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    let filename = entry.file_name().to_string_lossy().to_string();
                    
                    // Extract original filename (remove UUID prefix)
                    let display_name = if let Some(pos) = filename.find('_') {
                        filename[pos + 1..].to_string()
                    } else {
                        filename.clone()
                    };

                    let (file_type, can_preview) = get_file_type_and_preview(&display_name);

                    files.push(FileInfo {
                        name: display_name,
                        size: metadata.len(),
                        path: filename,
                        file_type,
                        can_preview,
                    });
                }
            }
        }
    }

    // Sort files by name
    files.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(HttpResponse::Ok().json(FileListResponse { files }))
}

// Delete a file
#[post("/delete/{filename}")]
async fn delete_file(path: web::Path<String>, session: actix_session::Session) -> ActixResult<HttpResponse> {
    require_auth(&session)?;
    let filename = path.into_inner();
    let filepath = PathBuf::from(UPLOAD_DIR).join(&filename);

    match std::fs::remove_file(&filepath) {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "File deleted successfully"
        }))),
        Err(_) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "message": "File not found"
        }))),
    }
}

// Preview text/code files
#[get("/preview/{filename}")]
async fn preview_file(path: web::Path<String>) -> ActixResult<HttpResponse> {
    let filename = path.into_inner();
    let filepath = PathBuf::from(UPLOAD_DIR).join(&filename);
    
    // Get original filename for type checking
    let display_name = if let Some(pos) = filename.find('_') {
        filename[pos + 1..].to_string()
    } else {
        filename.clone()
    };
    
    let (file_type, can_preview) = get_file_type_and_preview(&display_name);
    
    if !can_preview || (file_type != "text" && file_type != "code") {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "File cannot be previewed as text"
        })));
    }
    
    match std::fs::read_to_string(&filepath) {
        Ok(content) => {
            // Limit content size for preview (first 10KB)
            let preview_content = if content.len() > 10240 {
                format!("{}...\n\n[Content truncated - showing first 10KB of {}]", 
                    &content[..10240], display_name)
            } else {
                content
            };
            
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "content": preview_content,
                "type": file_type,
                "filename": display_name
            })))
        }
        Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to read file"
        })))
    }
}

fn get_disk_space(path: &str) -> (u64, u64) {
    // Try to get disk space information using `df` command
    // Returns (free_bytes, total_bytes)
    
    use std::process::Command;
    
    // Use df command to get disk space information
    if let Ok(output) = Command::new("df")
        .arg("-k") // Output in 1K blocks
        .arg(path)
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            // Parse df output - second line contains the data
            if let Some(line) = output_str.lines().nth(1) {
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() >= 4 {
                    // df -k outputs: filesystem, 1k-blocks, used, available, ...
                    if let (Ok(total_kb), Ok(avail_kb)) = (fields[1].parse::<u64>(), fields[3].parse::<u64>()) {
                        let total_bytes = total_kb * 1024;
                        let free_bytes = avail_kb * 1024;
                        return (free_bytes, total_bytes);
                    }
                }
            }
        }
    }
    
    // Fallback: return some reasonable defaults if we can't get disk info
    // This represents a 500GB disk with 250GB free as an example
    (250 * 1024 * 1024 * 1024, 500 * 1024 * 1024 * 1024)
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

fn sanitize_filename(filename: &str) -> String {
    // Remove path separators and other potentially dangerous characters
    filename
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect::<String>()
        .trim_start_matches('.')
        .to_string()
}

fn get_file_type_and_preview(filename: &str) -> (String, bool) {
    let extension = filename
        .rfind('.')
        .map(|i| filename[i + 1..].to_lowercase())
        .unwrap_or_default();
    
    match extension.as_str() {
        // Images
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "bmp" | "ico" => ("image".to_string(), true),
        // Videos
        "mp4" | "webm" | "mov" | "avi" | "mkv" | "m4v" => ("video".to_string(), true),
        // Audio (including ogg for audio)
        "mp3" | "wav" | "m4a" | "aac" | "flac" | "ogg" => ("audio".to_string(), true),
        // Text files
        "txt" | "md" | "json" | "xml" | "csv" | "log" | "yml" | "yaml" | "toml" | "ini" => ("text".to_string(), true),
        // Code files
        "js" | "ts" | "html" | "css" | "rs" | "py" | "java" | "c" | "cpp" | "h" | "hpp" | "go" | "rb" | "php" | "sh" | "bash" => ("code".to_string(), true),
        // PDFs
        "pdf" => ("pdf".to_string(), true),
        // Archives
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" => ("archive".to_string(), false),
        // Documents
        "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" => ("document".to_string(), false),
        // Default
        _ => ("unknown".to_string(), false),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    
    env_logger::init();

    // Create uploads directory if it doesn't exist
    create_dir_all(UPLOAD_DIR)?;

    println!("Starting file server at http://127.0.0.1:8080");
    println!("Upload directory: {}", UPLOAD_DIR);
    
    if args.debug {
        println!("Debug mode enabled");
    }

    let app_state = AppState {
        debug_mode: args.debug,
    };

    HttpServer::new(move || {
        // Generate a secret key for sessions (in production, use a persistent secret)
        let secret_key = Key::generate();
        
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .wrap(Logger::default())
            .wrap(
                SessionMiddleware::builder(
                    CookieSessionStore::default(),
                    secret_key,
                )
                .cookie_secure(false) // Set to true in production with HTTPS
                .build(),
            )
            .wrap(IdentityMiddleware::default())
            .service(index)
            .service(get_debug_info)
            .service(login)
            .service(logout)
            .service(auth_status)
            .service(upload_files)
            .service(list_files)
            .service(get_storage_info)
            .service(delete_file)
            .service(preview_file)
            // Serve uploaded files for download
            .service(fs::Files::new("/download", UPLOAD_DIR).show_files_listing())
            // Serve static files (CSS, JS)
            .service(fs::Files::new("/static", "./static"))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
