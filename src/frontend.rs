use leptos::*;
use wasm_bindgen::prelude::*;
use gloo_net::http::Request;
use gloo_file::{FileList, File};
use gloo_timers::future::TimeoutFuture;
use web_sys::{Event, FormData};

use crate::{FileInfo, FilesResponse, StorageInfo, ApiResponse, DebugInfo, LoginRequest, LoginResponse, AuthStatus};

#[component]
pub fn App() -> impl IntoView {
    let (files, set_files) = create_signal(Vec::<FileInfo>::new());
    let (storage_info, set_storage_info) = create_signal(None::<StorageInfo>);
    let (search_term, set_search_term) = create_signal(String::new());
    let (is_loading, set_is_loading) = create_signal(false);
    let (debug_mode, set_debug_mode) = create_signal(false);
    let (is_authenticated, set_is_authenticated) = create_signal(false);
    let (username, set_username) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());
    let (login_error, set_login_error) = create_signal(None::<String>);

    // Check authentication status on mount
    create_effect(move |_| {
        spawn_local(async move {
            check_auth_status(set_is_authenticated).await;
        });
    });

    // Load initial data when component mounts and user is authenticated
    create_effect(move |_| {
        if is_authenticated.get() {
            spawn_local(async move {
                // Small delay to ensure session is fully established
                TimeoutFuture::new(100).await;
                load_files_and_storage(set_files, set_storage_info, set_is_loading).await;
                load_debug_info(set_debug_mode).await;
            });
        }
    });

    // Create memo for filtered files
    let filtered_files = create_memo(move |_| {
        let search = search_term.get().trim().to_lowercase();
        let all_files = files.get();
        
        if search.is_empty() {
            all_files
        } else if search.starts_with("#") {
            let file_type = &search[1..];
            all_files.into_iter().filter(|file| {
                file.file_type.to_lowercase().contains(file_type)
            }).collect()
        } else {
            all_files.into_iter().filter(|file| {
                file.name.to_lowercase().contains(&search)
            }).collect()
        }
    });

    view! {
        <div class="app">
            <StyleProvider />
            <Show 
                when=move || is_authenticated.get()
                fallback=move || view! {
                    <LoginForm 
                        username=username
                        set_username=set_username
                        password=password
                        set_password=set_password
                        login_error=login_error
                        set_login_error=set_login_error
                        set_is_authenticated=set_is_authenticated
                    />
                }
            >
                <div class="main-grid">
                    <div class="header-section border-container">
                        <div style="display: flex; justify-content: space-between; align-items: center;">
                            <div>
                                <h1 style="color: #cdd6f4; margin: 0; font-size: 2.5rem; font-weight: 500;">
                                    "cratr"
                                </h1>
                                <p style="color: #bac2de; font-size: 1.1rem; margin: 10px 0 0 0;">
                                    "drag, drop, and manage your files with style"
                                </p>
                            </div>
                            <button 
                                type="button"
                                class="logout-btn border-container"
                                on:click=move |_| {
                                    spawn_local(async move {
                                        logout_user(set_is_authenticated).await;
                                    });
                                }
                            >
                                "logout"
                            </button>
                        </div>
                    </div>
                    
                    <div class="storage-section border-container">
                        <StorageSection storage_info=storage_info />
                    </div>
                    
                    <div class="upload-section border-container">
                        <UploadSection 
                            debug_mode=debug_mode
                            on_upload_complete=move || {
                                spawn_local(async move {
                                    load_files_and_storage(set_files, set_storage_info, set_is_loading).await;
                                });
                            }
                        />
                    </div>
                    
                    <div class="search-section border-container">
                        <SearchSection 
                            search_term=search_term
                            set_search_term=set_search_term
                        />
                    </div>
                    
                    <div class="files-section border-container">
                        <FilesSection 
                            files=filtered_files
                            is_loading=is_loading
                            set_files=set_files
                            set_storage_info=set_storage_info
                            set_is_loading=set_is_loading
                        />
                    </div>
                </div>
            </Show>
        </div>
    }
}

#[component]
pub fn StorageSection(
    storage_info: ReadSignal<Option<StorageInfo>>,
) -> impl IntoView {
    view! {
        <Show when=move || storage_info.get().is_some() fallback=|| view! { 
            <div style="color: #bac2de;">
                "loading storage info..."
            </div> 
        }>
            {move || {
                if let Some(info) = storage_info.get() {
                    view! {
                        <div class="storage-stats-grid">
                            <div class="stat-box border-container">
                                <div class="stat-value">{&info.formatted_used}</div>
                                <div class="stat-label">"used space"</div>
                            </div>
                            <div class="stat-box border-container">
                                <div class="stat-value">{info.total_files}</div>
                                <div class="stat-label">"total files"</div>
                            </div>
                            <div class="stat-box border-container">
                                <div class="stat-value">{format!("{:.1}%", info.used_percentage)}</div>
                                <div class="stat-label">"usage"</div>
                            </div>
                            
                            <div class="progress-section">
                                <div class="progress-bar">
                                    <div 
                                        class="progress-fill"
                                        style=format!("width: {:.1}%", info.used_percentage.min(100.0))
                                    ></div>
                                </div>
                                <div class="disk-info">
                                    "disk: " {&info.formatted_disk_free} " free of " {&info.formatted_disk_total}
                                </div>
                            </div>
                        </div>
                    }.into_view()
                } else {
                    view! { <div></div> }.into_view()
                }
            }}
        </Show>
    }
}

#[component]
pub fn LoginForm(
    username: ReadSignal<String>,
    set_username: WriteSignal<String>,
    password: ReadSignal<String>,
    set_password: WriteSignal<String>,
    login_error: ReadSignal<Option<String>>,
    set_login_error: WriteSignal<Option<String>>,
    set_is_authenticated: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="login-grid">
            <div class="login-header border-container">
                <h1 style="color: #cdd6f4; margin: 0 0 10px 0; font-size: 2.5rem; font-weight: 500;">
                    "cratr"
                </h1>
                <p style="color: #bac2de; font-size: 1.1rem; margin: 0;">
                    "secure file management system"
                </p>
            </div>
            
            <div class="login-form-section border-container">
                <Show when=move || login_error.get().is_some()>
                    <div class="login-error border-container">
                        {move || login_error.get().unwrap_or_default()}
                    </div>
                </Show>
                
                <form on:submit=move |e| {
                    e.prevent_default();
                    let username_val = username.get();
                    let password_val = password.get();
                    
                    spawn_local(async move {
                        set_login_error.set(None);
                        match login_user(&username_val, &password_val).await {
                            Ok(response) => {
                                if response.authenticated {
                                    set_is_authenticated.set(true);
                                } else {
                                    set_login_error.set(Some(response.message));
                                }
                            }
                            Err(e) => {
                                set_login_error.set(Some(format!("Login failed: {}", e)));
                            }
                        }
                    });
                }>
                    <div class="form-field">
                        <label class="field-label">"username"</label>
                        <input
                            type="text"
                            class="login-input username-input border-container"
                            prop:value=move || username.get()
                            on:input=move |e| set_username.set(event_target_value(&e))
                            placeholder="enter username"
                            required
                        />
                    </div>
                    
                    <div class="form-field">
                        <label class="field-label">"password"</label>
                        <input
                            type="password"
                            class="login-input password-input border-container"
                            prop:value=move || password.get()
                            on:input=move |e| set_password.set(event_target_value(&e))
                            placeholder="enter password"
                            required
                        />
                    </div>
                    
                    <div class="login-actions">
                        <button type="submit" class="login-btn border-container">
                            "authenticate"
                        </button>
                    </div>
                </form>
            </div>
            
            <div class="login-info border-container">
                <div class="info-section">
                    <h3 style="color: #cdd6f4; margin: 0 0 10px 0; font-size: 1.2rem;">
                        "default credentials"
                    </h3>
                    <div class="credential-info">
                        <div class="credential-item">
                            <span class="credential-label">"username:"</span>
                            <span class="credential-value border-container">"admin"</span>
                        </div>
                        <div class="credential-item">
                            <span class="credential-label">"password:"</span>
                            <span class="credential-value border-container">"admin"</span>
                        </div>
                    </div>
                </div>
                <div class="security-note">
                    <p style="color: #f38ba8; font-size: 14px; margin: 0;">
                        "⚠ change default credentials in production"
                    </p>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn SearchSection(
    search_term: ReadSignal<String>,
    set_search_term: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div>
            <input 
                type="text"
                class="search-input border-container"
                placeholder="search files..."
                prop:value=search_term
                on:input=move |ev| {
                    let value = event_target_value(&ev);
                    set_search_term.set(value);
                }
            />
            <div style="color: #6c7086; font-size: 12px; margin-top: 8px;">
                "use # to filter by type"
            </div>
        </div>
    }
}

#[component]
pub fn UploadSection<F>(
    debug_mode: ReadSignal<bool>,
    on_upload_complete: F,
) -> impl IntoView 
where
    F: Fn() + Copy + 'static,
{
    let (selected_files, set_selected_files) = create_signal(Vec::<File>::new());
    let (is_uploading, set_is_uploading) = create_signal(false);
    let file_input_ref = create_node_ref::<leptos::html::Input>();

    let on_file_change = move |_ev: Event| {
        web_sys::console::log_1(&"File input changed".into());
        if let Some(input) = file_input_ref.get_untracked() {
            if let Some(files) = input.files() {
                let file_list = FileList::from(files);
                let files_vec: Vec<File> = file_list.iter().cloned().collect();
                web_sys::console::log_1(&format!("Selected {} files", files_vec.len()).into());
                set_selected_files.set(files_vec);
            } else {
                web_sys::console::log_1(&"No files found".into());
            }
        } else {
            web_sys::console::log_1(&"Input ref not found".into());
        }
    };

    let on_choose_files_click = move |_| {
        if let Some(input) = file_input_ref.get_untracked() {
            input.click();
        }
    };

    let on_upload_click = move |_| {
        let files = selected_files.get();
        web_sys::console::log_1(&format!("Upload button clicked, files count: {}", files.len()).into());
        
        if files.is_empty() {
            web_sys::console::log_1(&"No files selected".into());
            return;
        }
        
        web_sys::console::log_1(&"Starting upload...".into());
        set_is_uploading.set(true);
        
        spawn_local(async move {
            web_sys::console::log_1(&"In spawn_local...".into());
            match upload_files(files).await {
                Ok(response) => {
                    web_sys::console::log_1(&format!("Upload successful: {:?}", response.message).into());
                    set_selected_files.set(Vec::new());
                    if let Some(input) = file_input_ref.get_untracked() {
                        input.set_value("");
                    }
                    // Call the callback to refresh the file list
                    web_sys::console::log_1(&"Calling upload complete callback...".into());
                    on_upload_complete();
                },
                Err(e) => {
                    web_sys::console::log_1(&format!("Upload failed: {}", e).into());
                    log::error!("Upload failed: {}", e);
                }
            }
            set_is_uploading.set(false);
        });
    };

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        web_sys::console::log_1(&"Form submitted".into());
        // Form submission will be handled by button click
    };

    view! {
        <div>
            <form on:submit=on_submit>
                <div style="margin-bottom: 15px;">
                    <input
                        type="file"
                        id="fileInput"
                        multiple
                        ref=file_input_ref
                        on:change=on_file_change
                        accept="*/*"
                        style="display: none;"
                    />
                    <button 
                        type="button"
                        class="choose-files-btn border-container"
                        on:click=on_choose_files_click
                    >
                        "choose files"
                    </button>
                </div>
                
                <Show when=move || !selected_files.get().is_empty()>
                    <div style="margin-bottom: 15px; text-align: left;">
                        <div style="color: #bac2de; font-size: 14px; margin-bottom: 8px;">
                            "selected:"
                        </div>
                        <div style="max-height: 80px; overflow-y: auto;">
                            <For
                                each=move || selected_files.get()
                                key=|file| file.name()
                                let:file
                            >
                                <div style="color: #a6adc8; font-size: 13px; margin: 2px 0;">
                                    {file.name()}
                                </div>
                            </For>
                        </div>
                    </div>
                </Show>
                
                <Show when=move || debug_mode.get()>
                    <div style="margin: 10px 0; color: #6c7086; font-size: 12px;">
                        "debug: " {move || selected_files.get().len()} " files | "
                        {move || if is_uploading.get() { "uploading..." } else { "ready" }}
                    </div>
                </Show>
                
                <button 
                    type="button"
                    class="upload-files-btn border-container"
                    disabled=move || selected_files.get().is_empty() || is_uploading.get()
                    on:click=on_upload_click
                >
                    {move || if is_uploading.get() { "uploading..." } else { "upload files" }}
                </button>
            </form>
        </div>
    }
}

#[component]
fn FileItem(
    file: FileInfo,
    set_files: WriteSignal<Vec<FileInfo>>,
    set_storage_info: WriteSignal<Option<StorageInfo>>,
    set_is_loading: WriteSignal<bool>,
) -> impl IntoView {
    let file_name = file.name.clone();
    let file_path = file.path.clone();
    let file_type = file.file_type.clone();
    let file_size = file.size;
    
    // Create multiple clones for different uses
    let file_path_preview = file_path.clone();
    let file_path_download = file_path.clone();
    let file_path_preview_btn = file_path.clone();
    let file_path_delete = file_path.clone();
    let file_type_preview_check = file_type.clone();
    let file_type_preview = file_type.clone();
    let file_type_preview_btn = file_type.clone();
    
    view! {
        <div class="file-item">
            <div style="display: flex; justify-content: space-between; align-items: start; margin-bottom: 15px;">
                <div style="color: #cdd6f4; font-weight: 500; word-break: break-word; flex: 1; margin-right: 10px;">
                    {&file_name}
                </div>
                <span 
                    class="file-type-badge"
                    style=format!("
                        color: {};
                        border-color: {};
                    ", 
                        get_file_type_color(&file_type),
                        get_file_type_color(&file_type)
                    )
                >
                    {&file_type}
                </span>
            </div>
            
            <Show when=move || is_previewable_file(&file_type_preview_check)>
                <div class="file-preview">
                    {
                        if file_type_preview == "image" {
                            view! {
                                <img 
                                    src=format!("/download/{}", file_path_preview)
                                    alt=file_name.clone()
                                    style="max-width: 100%; max-height: 250px; object-fit: contain;"
                                    loading="lazy"
                                />
                            }.into_view()
                        } else if file_type_preview == "video" {
                            view! {
                                <video 
                                    controls
                                    style="max-width: 100%; max-height: 250px;"
                                    preload="metadata"
                                >
                                    <source src=format!("/download/{}", file_path_preview) />
                                    "Your browser does not support the video tag."
                                </video>
                            }.into_view()
                        } else {
                            view! { <div></div> }.into_view()
                        }
                    }
                </div>
            </Show>
            
            <div style="color: #a6adc8; margin-bottom: 20px; font-size: 14px;">
                "size: " {format_file_size(file_size)}
            </div>
            
            <div style="display: flex; gap: 10px; flex-wrap: wrap; margin-top: auto;">
                <a 
                    href=format!("/download/{}", file_path_download)
                    class="action-btn border-container"
                    download
                >
                    "download"
                </a>
                
                <Show when=move || is_previewable_file(&file_type_preview_btn)>
                    <a 
                        href=format!("/download/{}", file_path_preview_btn)
                        class="action-btn border-container"
                        target="_blank"
                    >
                        "preview"
                    </a>
                </Show>
                
                <button
                    type="button"
                    class="action-btn delete-btn border-container"
                    on:click={
                        move |e| {
                            e.prevent_default();
                            let file_path = file_path_delete.clone();
                            spawn_local(async move {
                                match Request::post(&format!("/delete/{}", file_path)).send().await {
                                    Ok(_) => {
                                        spawn_local(async move {
                                            load_files_and_storage(set_files, set_storage_info, set_is_loading).await;
                                        });
                                    }
                                    Err(e) => {
                                        web_sys::console::log_1(&format!("Delete failed: {}", e).into());
                                    }
                                }
                            });
                        }
                    }
                >
                    "delete"
                </button>
            </div>
        </div>
    }
}

#[component]
fn FilesSection(
    files: Memo<Vec<FileInfo>>,
    is_loading: ReadSignal<bool>,
    set_files: WriteSignal<Vec<FileInfo>>,
    set_storage_info: WriteSignal<Option<StorageInfo>>,
    set_is_loading: WriteSignal<bool>,
) -> impl IntoView 
{
    view! {
        <div>
            <Show 
                when=move || is_loading.get()
                fallback=move || {
                    view! {
                        <div>
                            <Show
                                when=move || !files.get().is_empty()
                                fallback=move || {
                                    view! {
                                        <div style="
                                            text-align: center;
                                            padding: 40px 20px;
                                            color: #bac2de;
                                        ">
                                            <div style="font-size: 32px; margin-bottom: 10px;">"[ ]"</div>
                                            <div>"no files uploaded yet"</div>
                                            <div style="color: #6c7086; font-size: 14px; margin-top: 5px;">
                                                "upload some files to get started"
                                            </div>
                                        </div>
                                    }
                                }
                            >
                                <div class="files-grid">
                                    <For
                                        each=move || files.get()
                                        key=|file| file.path.clone()
                                        let:file
                                    >
                                        <FileItem 
                                            file=file 
                                            set_files=set_files
                                            set_storage_info=set_storage_info 
                                            set_is_loading=set_is_loading
                                        />
                                    </For>
                                </div>
                            </Show>
                        </div>
                    }
                }
            >
                <div style="text-align: center; color: #bac2de; padding: 20px;">
                    "loading files..."
                </div>
            </Show>
        </div>
    }
}

async fn load_debug_info(set_debug_mode: WriteSignal<bool>) {
    match Request::get("/debug").send().await {
        Ok(response) => {
            if let Ok(debug_info) = response.json::<DebugInfo>().await {
                set_debug_mode.set(debug_info.debug_mode);
            }
        },
        Err(_) => {
            // Default to false if we can't get debug info
            set_debug_mode.set(false);
        }
    }
}

async fn load_files_and_storage(
    set_files: WriteSignal<Vec<FileInfo>>,
    set_storage_info: WriteSignal<Option<StorageInfo>>,
    set_is_loading: WriteSignal<bool>,
) {
    web_sys::console::log_1(&"Loading files and storage...".into());
    set_is_loading.set(true);
    
    // Make requests individually with better error handling
    let files_result = async {
        web_sys::console::log_1(&"Requesting files...".into());
        match Request::get("/files").send().await {
            Ok(response) => {
                if response.status() == 200 {
                    response.json::<FilesResponse>().await.map_err(|e| format!("Failed to parse files response: {:?}", e))
                } else {
                    Err(format!("Files request failed with status: {}", response.status()))
                }
            }
            Err(e) => Err(format!("Files request failed: {:?}", e))
        }
    }.await;
    
    let storage_result = async {
        web_sys::console::log_1(&"Requesting storage info...".into());
        match Request::get("/storage").send().await {
            Ok(response) => {
                if response.status() == 200 {
                    response.json::<StorageInfo>().await.map_err(|e| format!("Failed to parse storage response: {:?}", e))
                } else {
                    Err(format!("Storage request failed with status: {}", response.status()))
                }
            }
            Err(e) => Err(format!("Storage request failed: {:?}", e))
        }
    }.await;
    
    // Handle results separately
    match files_result {
        Ok(files_response) => {
            web_sys::console::log_1(&format!("Loaded {} files", files_response.files.len()).into());
            set_files.set(files_response.files);
        },
        Err(e) => {
            web_sys::console::log_1(&format!("Error loading files: {}", e).into());
            set_files.set(Vec::new());
        }
    }
    
    match storage_result {
        Ok(storage_response) => {
            web_sys::console::log_1(&format!("Storage: {} free", storage_response.formatted_disk_free).into());
            set_storage_info.set(Some(storage_response));
        },
        Err(e) => {
            web_sys::console::log_1(&format!("Error loading storage: {}", e).into());
            set_storage_info.set(None);
        }
    }
    
    set_is_loading.set(false);
    web_sys::console::log_1(&"Finished loading files and storage".into());
}

async fn upload_files(files: Vec<File>) -> Result<ApiResponse, String> {
    let form_data = FormData::new().map_err(|_| "Failed to create FormData")?;
    
    for file in files {
        form_data.append_with_blob("files", &file.as_ref())
            .map_err(|_| "Failed to append file to FormData")?;
    }
    
    let response = Request::post("/upload")
        .body(form_data)
        .map_err(|e| format!("Failed to set body: {:?}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {:?}", e))?;
        
    response.json::<ApiResponse>().await
        .map_err(|e| format!("Failed to parse response: {:?}", e))
}

async fn delete_file_api(filename: &str) -> Result<ApiResponse, String> {
    let response = Request::post(&format!("/delete/{}", filename))
        .send()
        .await
        .map_err(|e| format!("Request failed: {:?}", e))?;
        
    response.json::<ApiResponse>().await
        .map_err(|e| format!("Failed to parse response: {:?}", e))
}

async fn check_auth_status(set_is_authenticated: WriteSignal<bool>) {
    web_sys::console::log_1(&"Checking authentication status...".into());
    match Request::get("/auth/status").send().await {
        Ok(response) => {
            if response.status() == 200 {
                match response.json::<AuthStatus>().await {
                    Ok(auth_status) => {
                        web_sys::console::log_1(&format!("Auth status: authenticated={}", auth_status.authenticated).into());
                        set_is_authenticated.set(auth_status.authenticated);
                    }
                    Err(e) => {
                        web_sys::console::log_1(&format!("Failed to parse auth response: {:?}", e).into());
                        set_is_authenticated.set(false);
                    }
                }
            } else {
                web_sys::console::log_1(&format!("Auth status request failed with status: {}", response.status()).into());
                set_is_authenticated.set(false);
            }
        }
        Err(e) => {
            web_sys::console::log_1(&format!("Auth status request failed: {:?}", e).into());
            set_is_authenticated.set(false);
        }
    }
}

async fn login_user(username: &str, password: &str) -> Result<LoginResponse, String> {
    let login_request = LoginRequest {
        username: username.to_string(),
        password: password.to_string(),
    };
    
    let request_body = serde_json::to_string(&login_request)
        .map_err(|e| format!("Serialization error: {:?}", e))?;
    
    let response = Request::post("/login")
        .header("Content-Type", "application/json")
        .body(request_body)
        .map_err(|e| format!("Request body error: {:?}", e))?
        .send()
        .await
        .map_err(|e| format!("Login request failed: {:?}", e))?;
        
    response.json::<LoginResponse>().await
        .map_err(|e| format!("Failed to parse login response: {:?}", e))
}

async fn logout_user(set_is_authenticated: WriteSignal<bool>) {
    match Request::post("/logout").send().await {
        Ok(_) => {
            set_is_authenticated.set(false);
        }
        Err(e) => {
            web_sys::console::log_1(&format!("Logout failed: {:?}", e).into());
        }
    }
}

fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

fn get_file_type_color(file_type: &str) -> &'static str {
    match file_type {
        "image" => "#a6e3a1",  // Catppuccin green
        "video" => "#f38ba8",  // Catppuccin pink  
        "audio" => "#cba6f7",  // Catppuccin mauve
        "text" | "code" => "#89b4fa", // Catppuccin blue
        "pdf" => "#fab387",    // Catppuccin peach
        "archive" => "#f9e2af", // Catppuccin yellow
        _ => "#6c7086"         // Catppuccin overlay1
    }
}

fn is_previewable_file(file_type: &str) -> bool {
    matches!(file_type, "image" | "video")
}

#[wasm_bindgen]
pub fn run() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App /> });
}

// CSS-in-Rust: Define styles as const strings with Catppuccin Mocha and grid design
const MAIN_STYLES: &str = r#"
@import url("https://fonts.googleapis.com/css2?family=DM+Mono:ital,wght@0,300;0,400;0,500&display=swap");

body {
    font-family: "DM Mono", monospace;
    letter-spacing: -0.05ch;
    background-color: #1e1e2e;
    color: #cdd6f4;
    user-select: none;
    margin: 0;
    padding: 20px;
}

.app {
    max-width: 1200px;
    margin: 0 auto;
}

.main-grid {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    grid-template-rows: auto auto auto auto;
    gap: 20px;
    margin: 20px 0;
}

.border-container {
    position: relative;
    padding: 20px;
    border: 2px solid #45475a;
    transition: border-color 0.2s ease-out;
    text-align: center;
    background-color: #1e1e2e;
}

.border-container::before {
    position: absolute;
    top: -12px;
    left: 20px;
    background-color: #1e1e2e;
    padding: 0 8px;
    font-size: 16px;
    color: #45475a;
    transition: color 0.2s ease-out;
}

.header-section {
    grid-column: 1 / span 6;
    grid-row: 1;
}
.header-section::before {
    content: "file upload system";
}
.header-section:hover {
    border-color: #cba6f7;
}
.header-section:hover::before {
    color: #cba6f7;
}

.storage-section {
    grid-column: 1 / span 3;
    grid-row: 2;
}
.storage-section::before {
    content: "storage info";
}
.storage-section:hover {
    border-color: #89b4fa;
}
.storage-section:hover::before {
    color: #89b4fa;
}

.upload-section {
    grid-column: 4 / span 3;
    grid-row: 2;
}
.upload-section::before {
    content: "upload files";
}
.upload-section:hover {
    border-color: #a6e3a1;
}
.upload-section:hover::before {
    color: #a6e3a1;
}

.search-section {
    grid-column: 1 / span 2;
    grid-row: 3;
}
.search-section::before {
    content: "search";
}
.search-section:hover {
    border-color: #fab387;
}
.search-section:hover::before {
    color: #fab387;
}

.files-section {
    grid-column: 1 / span 6;
    grid-row: 4;
}
.files-section::before {
    content: "files";
}
.files-section:hover {
    border-color: #f38ba8;
}
.files-section:hover::before {
    color: #f38ba8;
}

.choose-files-btn, .upload-files-btn {
    background-color: #1e1e2e;
    border: 2px solid #45475a;
    color: #cdd6f4;
    padding: 20px 20px 10px 20px;
    cursor: pointer;
    font-family: "DM Mono", monospace;
    font-size: 16px;
    transition: border-color 0.2s ease-out;
    position: relative;
    margin: 5px;
}

.choose-files-btn.border-container::before {
    content: "btn";
    position: absolute;
    top: -12px;
    left: 10px;
    background-color: #1e1e2e;
    padding: 0 8px;
    font-size: 12px;
    color: #45475a;
    transition: color 0.2s ease-out;
}

.upload-files-btn.border-container::before {
    content: "btn";
    position: absolute;
    top: -12px;
    left: 10px;
    background-color: #1e1e2e;
    padding: 0 8px;
    font-size: 12px;
    color: #45475a;
    transition: color 0.2s ease-out;
}

.choose-files-btn:hover, .upload-files-btn:hover:not(:disabled) {
    border-color: #a6e3a1;
}

.choose-files-btn:hover.border-container::before, .upload-files-btn:hover:not(:disabled).border-container::before {
    color: #a6e3a1;
}

.upload-files-btn:disabled {
    border-color: #313244;
    color: #6c7086;
    cursor: not-allowed;
}

.upload-files-btn:disabled.border-container::before {
    color: #313244;
}

.file-item {
    background-color: #1e1e2e;
    border: 2px solid #45475a;
    padding: 20px;
    margin: 10px 0;
    transition: border-color 0.2s ease-out;
    position: relative;
    min-height: 300px;
}

.file-item::before {
    content: "file";
    position: absolute;
    top: -12px;
    left: 20px;
    background-color: #1e1e2e;
    padding: 0 8px;
    font-size: 14px;
    color: #45475a;
    transition: color 0.2s ease-out;
}

.file-item:hover {
    border-color: #f38ba8;
}

.file-item:hover::before {
    color: #f38ba8;
}

.search-input {
    background-color: #1e1e2e;
    border: 2px solid #45475a;
    color: #cdd6f4;
    padding: 20px 15px 10px 15px;
    font-family: "DM Mono", monospace;
    font-size: 16px;
    width: calc(100% - 34px);
    transition: border-color 0.2s ease-out;
    box-sizing: border-box;
    position: relative;
}

.search-input.border-container::before {
    content: "input";
    position: absolute;
    top: -12px;
    left: 15px;
    background-color: #1e1e2e;
    padding: 0 8px;
    font-size: 12px;
    color: #45475a;
    transition: color 0.2s ease-out;
}

.search-input:focus {
    outline: none;
    border-color: #fab387;
}

.search-input:focus.border-container::before {
    color: #fab387;
}

.search-input::placeholder {
    color: #6c7086;
}

.files-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(400px, 1fr));
    gap: 20px;
    margin-top: 20px;
}

.action-btn {
    background-color: #1e1e2e;
    border: 2px solid #45475a;
    color: #cdd6f4;
    padding: 20px 16px 8px 16px;
    cursor: pointer;
    font-family: "DM Mono", monospace;
    font-size: 14px;
    transition: border-color 0.2s ease-out;
    margin: 4px;
    text-decoration: none;
    display: inline-block;
    position: relative;
}

.action-btn.border-container::before {
    content: "btn";
    position: absolute;
    top: -12px;
    left: 10px;
    background-color: #1e1e2e;
    padding: 0 8px;
    font-size: 12px;
    color: #45475a;
    transition: color 0.2s ease-out;
}

.action-btn:hover {
    border-color: #89b4fa;
    color: #cdd6f4;
    text-decoration: none;
}

.action-btn:hover.border-container::before {
    color: #89b4fa;
}

.delete-btn.border-container::before {
    content: "del";
}

.delete-btn:hover {
    border-color: #f38ba8;
}

.delete-btn:hover.border-container::before {
    color: #f38ba8;
}

.logout-btn {
    background-color: #1e1e2e;
    border: 2px solid #45475a;
    color: #cdd6f4;
    padding: 20px 16px 8px 16px;
    cursor: pointer;
    font-family: "DM Mono", monospace;
    font-size: 14px;
    transition: border-color 0.2s ease-out;
    margin: 4px;
    position: relative;
}

.logout-btn.border-container::before {
    content: "logout";
    position: absolute;
    top: -12px;
    left: 10px;
    background-color: #1e1e2e;
    padding: 0 8px;
    font-size: 12px;
    color: #45475a;
    transition: color 0.2s ease-out;
}

.logout-btn:hover {
    border-color: #f38ba8;
}

.logout-btn:hover.border-container::before {
    color: #f38ba8;
}

.storage-stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 15px;
    margin: 15px 0;
    text-align: left;
}

.stat-item {
    color: #bac2de;
    font-size: 14px;
}

.stat-value {
    color: #cdd6f4;
    font-weight: 500;
    font-size: 16px;
}

.storage-stats-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    grid-template-rows: auto auto;
    gap: 15px;
    margin: 15px 0;
}

.stat-box {
    text-align: center;
    padding: 15px;
    position: relative;
}

.stat-box::before {
    content: "";
}

.stat-box:nth-child(1)::before {
    content: "used";
    position: absolute;
    top: -12px;
    left: 15px;
    background-color: #1e1e2e;
    padding: 0 8px;
    font-size: 12px;
    color: #45475a;
    transition: color 0.2s ease-out;
}

.stat-box:nth-child(2)::before {
    content: "files";
    position: absolute;
    top: -12px;
    left: 15px;
    background-color: #1e1e2e;
    padding: 0 8px;
    font-size: 12px;
    color: #45475a;
    transition: color 0.2s ease-out;
}

.stat-box:nth-child(3)::before {
    content: "usage";
    position: absolute;
    top: -12px;
    left: 15px;
    background-color: #1e1e2e;
    padding: 0 8px;
    font-size: 12px;
    color: #45475a;
    transition: color 0.2s ease-out;
}

.stat-box:nth-child(1):hover {
    border-color: #a6e3a1;
}

.stat-box:nth-child(1):hover::before {
    color: #a6e3a1;
}

.stat-box:nth-child(2):hover {
    border-color: #89b4fa;
}

.stat-box:nth-child(2):hover::before {
    color: #89b4fa;
}

.stat-box:nth-child(3):hover {
    border-color: #f38ba8;
}

.stat-box:nth-child(3):hover::before {
    color: #f38ba8;
}

.stat-box .stat-value {
    color: #cdd6f4;
    font-weight: 500;
    font-size: 18px;
    margin-bottom: 5px;
}

.stat-box .stat-label {
    color: #bac2de;
    font-size: 12px;
    text-transform: lowercase;
}

.progress-section {
    grid-column: 1 / span 3;
    padding: 15px 0;
}

.disk-info {
    margin-top: 10px;
    font-size: 12px;
    color: #a6adc8;
    text-align: center;
}

.progress-bar {
    width: 100%;
    background-color: #313244;
    height: 8px;
    margin: 10px 0;
}

.progress-fill {
    height: 100%;
    background-color: #89b4fa;
    transition: width 0.75s ease;
}

.file-type-badge {
    font-size: 12px;
    padding: 2px 6px;
    border: 1px solid;
    text-transform: uppercase;
    font-weight: 500;
}

.file-preview {
    display: flex;
    justify-content: center;
    align-items: center;
    border: 1px solid #45475a;
    border-radius: 8px;
    overflow: hidden;
    background-color: #1e1e2e;
    min-height: 180px;
    margin-bottom: 15px;
}

.file-preview img,
.file-preview video {
    max-width: 100%;
    max-height: 250px;
    object-fit: contain;
    border-radius: 6px;
}

.file-preview video {
    width: 100%;
}

/* Responsive design */
@media (max-width: 768px) {
    .main-grid {
        grid-template-columns: 1fr;
        grid-template-rows: auto;
    }
    
    .header-section, .storage-section, .upload-section, 
    .search-section, .files-section {
        grid-column: 1;
    }
    
    .files-grid {
        grid-template-columns: 1fr;
    }
}

/* Login Form Styles - Matching site design language */
.login-grid {
    display: grid;
    grid-template-columns: 1fr;
    grid-template-rows: auto auto auto;
    gap: 20px;
    padding: 40px 20px;
    max-width: 500px;
    margin: 0 auto;
    min-height: 100vh;
    align-content: center;
}

.login-header {
    text-align: center;
    padding: 30px;
}

.login-header::before {
    content: "system";
}

.login-header:hover {
    border-color: #cba6f7;
}

.login-header:hover::before {
    color: #cba6f7;
}

.login-form-section {
    padding: 30px;
}

.login-form-section::before {
    content: "authenticate";
}

.login-form-section:hover {
    border-color: #89b4fa;
}

.login-form-section:hover::before {
    color: #89b4fa;
}

.login-info {
    padding: 25px;
}

.login-info::before {
    content: "credentials";
}

.login-info:hover {
    border-color: #a6e3a1;
}

.login-info:hover::before {
    color: #a6e3a1;
}

.form-field {
    margin-bottom: 20px;
}

.field-label {
    display: block;
    color: #cdd6f4;
    font-size: 14px;
    font-weight: 500;
    margin-bottom: 8px;
    text-transform: lowercase;
}

.login-input {
    width: 100%;
    background-color: #1e1e2e;
    border: 2px solid #45475a;
    color: #cdd6f4;
    padding: 12px 16px;
    font-family: "DM Mono", monospace;
    font-size: 16px;
    border-radius: 4px;
    transition: border-color 0.2s ease-out;
    box-sizing: border-box;
    position: relative;
}

.login-input.border-container {
    border-radius: 0;
    padding: 20px 16px 12px 16px;
}

.username-input.border-container::before {
    content: "username";
    position: absolute;
    top: -12px;
    left: 15px;
    background-color: #1e1e2e;
    padding: 0 8px;
    font-size: 12px;
    color: #45475a;
    transition: color 0.2s ease-out;
}

.password-input.border-container::before {
    content: "password";
    position: absolute;
    top: -12px;
    left: 15px;
    background-color: #1e1e2e;
    padding: 0 8px;
    font-size: 12px;
    color: #45475a;
    transition: color 0.2s ease-out;
}

.login-input:focus {
    outline: none;
    border-color: #89b4fa;
}

.login-input:focus.border-container::before {
    color: #89b4fa;
}

.login-input:hover:not(:focus) {
    border-color: #6c7086;
}

.login-input:hover:not(:focus).border-container::before {
    color: #6c7086;
}

.login-input::placeholder {
    color: #6c7086;
    font-style: italic;
}

.login-actions {
    margin-top: 25px;
}

.login-btn {
    width: 100%;
    background-color: #1e1e2e;
    border: 2px solid #45475a;
    color: #cdd6f4;
    padding: 14px 20px;
    font-family: "DM Mono", monospace;
    font-size: 16px;
    font-weight: 500;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.2s ease-out;
    text-transform: lowercase;
    position: relative;
}

.login-btn.border-container {
    border-radius: 0;
    padding: 20px 20px 14px 20px;
}

.login-btn.border-container::before {
    content: "auth";
    position: absolute;
    top: -12px;
    left: 15px;
    background-color: #1e1e2e;
    padding: 0 8px;
    font-size: 12px;
    color: #45475a;
    transition: color 0.2s ease-out;
}

.login-btn:hover {
    border-color: #89b4fa;
    transform: translateY(-1px);
}

.login-btn:hover.border-container::before {
    color: #89b4fa;
}

.login-error {
    background-color: #1e1e2e;
    color: #f38ba8;
    padding: 20px 16px 12px 16px;
    margin-bottom: 20px;
    font-size: 14px;
    font-weight: 500;
    border: 2px solid #f38ba8;
    position: relative;
}

.login-error.border-container::before {
    content: "error";
    position: absolute;
    top: -12px;
    left: 15px;
    background-color: #1e1e2e;
    padding: 0 8px;
    font-size: 12px;
    color: #f38ba8;
    transition: color 0.2s ease-out;
}

.info-section {
    margin-bottom: 20px;
}

.credential-info {
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.credential-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 0;
}

.credential-label {
    color: #bac2de;
    font-size: 14px;
}

.credential-value {
    color: #a6e3a1;
    font-family: "DM Mono", monospace;
    font-weight: 500;
    background-color: #1e1e2e;
    padding: 4px 8px;
    border-radius: 3px;
    border: 1px solid #45475a;
    position: relative;
}

.credential-value.border-container {
    border: 2px solid #45475a;
    border-radius: 0;
    padding: 12px 8px 4px 8px;
    transition: border-color 0.2s ease-out;
}

.credential-value.border-container:nth-of-type(2)::before {
    content: "user";
    position: absolute;
    top: -10px;
    left: 5px;
    background-color: #1e1e2e;
    padding: 0 4px;
    font-size: 10px;
    color: #45475a;
    transition: color 0.2s ease-out;
}

.credential-value.border-container:nth-of-type(4)::before {
    content: "pass";
    position: absolute;
    top: -10px;
    left: 5px;
    background-color: #1e1e2e;
    padding: 0 4px;
    font-size: 10px;
    color: #45475a;
    transition: color 0.2s ease-out;
}

.credential-value.border-container:hover {
    border-color: #a6e3a1;
}

.credential-value.border-container:hover::before {
    color: #a6e3a1;
}

.security-note {
    border-top: 1px solid #45475a;
    padding-top: 15px;
}

@media (max-width: 768px) {
    .login-grid {
        padding: 20px 15px;
        gap: 15px;
    }
    
    .login-header,
    .login-form-section,
    .login-info {
        padding: 20px;
    }
}
}
"#;

// CSS-in-Rust: Component that injects styles
#[component]
fn StyleProvider() -> impl IntoView {
    view! {
        <style>{MAIN_STYLES}</style>
    }
}
