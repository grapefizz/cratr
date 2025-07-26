use leptos::*;
use wasm_bindgen::prelude::*;
use gloo_net::http::Request;
use gloo_file::{FileList, File};
use web_sys::{Event, FormData};

use crate::{FileInfo, FilesResponse, StorageInfo, ApiResponse, DebugInfo};

#[component]
pub fn App() -> impl IntoView {
    let (files, set_files) = create_signal(Vec::<FileInfo>::new());
    let (storage_info, set_storage_info) = create_signal(None::<StorageInfo>);
    let (search_term, set_search_term) = create_signal(String::new());
    let (is_loading, set_is_loading) = create_signal(false);
    let (debug_mode, set_debug_mode) = create_signal(false);

    // Load initial data when component mounts
    create_effect(move |_| {
        spawn_local(async move {
            load_files_and_storage(set_files, set_storage_info, set_is_loading).await;
            load_debug_info(set_debug_mode).await;
        });
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
            <div class="main-grid">
                <div class="header-section border-container">
                    <h1 style="color: #cdd6f4; margin: 0; font-size: 2.5rem; font-weight: 500;">
                        "cratr"
                    </h1>
                    <p style="color: #bac2de; font-size: 1.1rem; margin: 10px 0 0 0;">
                        "drag, drop, and manage your files with style"
                    </p>
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
                        <div>
                            <div class="storage-stats">
                                <div class="stat-item">
                                    "used space"
                                    <div class="stat-value">{&info.formatted_used}</div>
                                </div>
                                <div class="stat-item">
                                    "total files"
                                    <div class="stat-value">{info.total_files}</div>
                                </div>
                                <div class="stat-item">
                                    "usage"
                                    <div class="stat-value">{format!("{:.1}%", info.used_percentage)}</div>
                                </div>
                            </div>
                            
                            <div class="progress-bar">
                                <div 
                                    class="progress-fill"
                                    style=format!("width: {:.1}%", info.used_percentage.min(100.0))
                                ></div>
                            </div>
                            
                            <div style="margin-top: 15px; font-size: 14px; color: #a6adc8;">
                                "disk: " {&info.formatted_disk_free} " free of " {&info.formatted_disk_total}
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
pub fn SearchSection(
    search_term: ReadSignal<String>,
    set_search_term: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div>
            <input 
                type="text"
                class="search-input"
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
                        class="choose-files-btn"
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
                    class="upload-files-btn"
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
                    class="action-btn"
                    download
                >
                    "download"
                </a>
                
                <Show when=move || is_previewable_file(&file_type_preview_btn)>
                    <a 
                        href=format!("/download/{}", file_path_preview_btn)
                        class="action-btn"
                        target="_blank"
                    >
                        "preview"
                    </a>
                </Show>
                
                <button
                    class="action-btn delete-btn"
                    on:click={
                        move |_| {
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
    
    // Make requests concurrently
    let files_request = async {
        web_sys::console::log_1(&"Requesting files...".into());
        let response = Request::get("/files").send().await?;
        response.json::<FilesResponse>().await
    };
    
    let storage_request = async {
        web_sys::console::log_1(&"Requesting storage info...".into());
        let response = Request::get("/storage").send().await?;
        response.json::<StorageInfo>().await
    };
    
    // Wait for both requests
    match futures::try_join!(files_request, storage_request) {
        Ok((files_response, storage_response)) => {
            web_sys::console::log_1(&format!("Loaded {} files", files_response.files.len()).into());
            web_sys::console::log_1(&format!("Storage: {} free", storage_response.formatted_disk_free).into());
            set_files.set(files_response.files);
            set_storage_info.set(Some(storage_response));
        },
        Err(e) => {
            web_sys::console::log_1(&format!("Error loading data: {:?}", e).into());
            // Set empty data on error
            set_files.set(Vec::new());
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
    padding: 10px 20px;
    cursor: pointer;
    font-family: "DM Mono", monospace;
    font-size: 16px;
    transition: border-color 0.2s ease-out;
    position: relative;
    margin: 5px;
}

.choose-files-btn::before {
    content: "choose";
    position: absolute;
    top: -12px;
    left: 10px;
    background-color: #1e1e2e;
    padding: 0 8px;
    font-size: 14px;
    color: #45475a;
    transition: color 0.2s ease-out;
}

.upload-files-btn::before {
    content: "upload";
    position: absolute;
    top: -12px;
    left: 10px;
    background-color: #1e1e2e;
    padding: 0 8px;
    font-size: 14px;
    color: #45475a;
    transition: color 0.2s ease-out;
}

.choose-files-btn:hover, .upload-files-btn:hover:not(:disabled) {
    border-color: #a6e3a1;
}

.choose-files-btn:hover::before, .upload-files-btn:hover:not(:disabled)::before {
    color: #a6e3a1;
}

.upload-files-btn:disabled {
    border-color: #313244;
    color: #6c7086;
    cursor: not-allowed;
}

.upload-files-btn:disabled::before {
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
    background-color: #11111b;
    border: 2px solid #45475a;
    color: #cdd6f4;
    padding: 10px 15px;
    font-family: "DM Mono", monospace;
    font-size: 16px;
    width: calc(100% - 34px);
    transition: border-color 0.2s ease-out;
    box-sizing: border-box;
}

.search-input:focus {
    outline: none;
    border-color: #fab387;
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
    background-color: #11111b;
    border: 2px solid #45475a;
    color: #cdd6f4;
    padding: 8px 16px;
    cursor: pointer;
    font-family: "DM Mono", monospace;
    font-size: 14px;
    transition: border-color 0.2s ease-out;
    margin: 4px;
    text-decoration: none;
    display: inline-block;
}

.action-btn:hover {
    border-color: #89b4fa;
    color: #cdd6f4;
    text-decoration: none;
}

.delete-btn {
    border-color: #45475a;
}

.delete-btn:hover {
    border-color: #f38ba8;
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
    background-color: #11111b;
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
"#;

// CSS-in-Rust: Component that injects styles
#[component]
fn StyleProvider() -> impl IntoView {
    view! {
        <style>{MAIN_STYLES}</style>
    }
}
