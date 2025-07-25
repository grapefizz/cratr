use leptos::*;
use wasm_bindgen::prelude::*;
use gloo_net::http::Request;
use gloo_file::{FileList, File};
use web_sys::{Event, FormData};
use std::rc::Rc;

use crate::{FileInfo, FilesResponse, StorageInfo, ApiResponse, PreviewResponse};

#[component]
pub fn App() -> impl IntoView {
    let (files, set_files) = create_signal::<Vec<FileInfo>>(vec![]);
    let (storage_info, set_storage_info) = create_signal::<Option<StorageInfo>>(None);
    let (selected_files, set_selected_files) = create_signal::<Vec<File>>(vec![]);
    let (search_term, set_search_term) = create_signal::<String>(String::new());
    let (upload_message, set_upload_message) = create_signal::<Option<(String, String)>>(None);
    let (is_loading, set_is_loading) = create_signal(false);
    let (is_uploading, set_is_uploading) = create_signal(false);

    // Load files and storage info on component mount
    create_effect(move |_| {
        spawn_local(async move {
            load_files_and_storage(set_files, set_storage_info, set_is_loading).await;
        });
    });

    // Filter files based on search term
    let filtered_files = create_memo(move |_| {
        let search = search_term.get().trim().to_lowercase();
        let all_files = files.get();
        
        if search.is_empty() {
            all_files
        } else if search.starts_with('#') {
            let file_type = search.trim_start_matches('#');
            all_files.into_iter()
                .filter(|file| file.file_type == file_type)
                .collect()
        } else {
            all_files.into_iter()
                .filter(|file| file.name.to_lowercase().contains(&search))
                .collect()
        }
    });

    view! {
        <div class="container">
            <Header />
            <StorageSection storage_info=storage_info />
            <UploadSection 
                selected_files=selected_files 
                set_selected_files=set_selected_files
                upload_message=upload_message
                set_upload_message=set_upload_message
                is_uploading=is_uploading
                set_is_uploading=set_is_uploading
                on_upload_success=move || {
                    spawn_local(async move {
                        load_files_and_storage(set_files, set_storage_info, set_is_loading).await;
                    });
                }
            />
            <FilesSection 
                files=filtered_files
                search_term=search_term
                set_search_term=set_search_term
                is_loading=is_loading
                on_delete=move |filename: String| {
                    spawn_local(async move {
                        if let Ok(_) = delete_file(&filename).await {
                            load_files_and_storage(set_files, set_storage_info, set_is_loading).await;
                        }
                    });
                }
            />
        </div>
    }
}

#[component]
fn Header() -> impl IntoView {
    view! {
        <div class="header">
            <h1>"file server"</h1>
            <p>"simple and secure file server for linux systems"</p>
        </div>
    }
}

#[component]
fn StorageSection(storage_info: ReadSignal<Option<StorageInfo>>) -> impl IntoView {
    view! {
        <div class="storage-section">
            <div class="storage-info">
                <div class="storage-stats">
                    <div class="storage-label">"server disk space:"</div>
                    <div class="storage-stats-row">
                        {move || match storage_info.get() {
                            Some(info) => view! {
                                <span>{format!("{} free", info.formatted_disk_free)}</span>
                                <span>{format!("{} total", info.formatted_disk_total)}</span>
                                <span>{format!("{}% used", info.disk_used_percentage.round() as u32)}</span>
                            }.into_view(),
                            None => view! {
                                <span>"0 B free"</span>
                                <span>"0 B total"</span>
                                <span>"0% used"</span>
                            }.into_view()
                        }}
                    </div>
                </div>
            </div>
            
            <div class="storage-bar-container">
                <div class="bar-label">"disk usage"</div>
                <div class="storage-bar-wrapper">
                    <div 
                        class="disk-bar"
                        style:width=move || {
                            storage_info.get()
                                .map(|info| format!("{}%", info.disk_used_percentage.min(100.0)))
                                .unwrap_or_else(|| "0%".to_string())
                        }
                    ></div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn UploadSection<F>(
    selected_files: ReadSignal<Vec<File>>,
    set_selected_files: WriteSignal<Vec<File>>,
    upload_message: ReadSignal<Option<(String, String)>>,
    set_upload_message: WriteSignal<Option<(String, String)>>,
    is_uploading: ReadSignal<bool>,
    set_is_uploading: WriteSignal<bool>,
    on_upload_success: F,
) -> impl IntoView 
where 
    F: Fn() + Clone + 'static,
{
    let file_input_ref = create_node_ref::<html::Input>();
    
    let on_file_change = move |ev: Event| {
        if let Some(input) = file_input_ref.get() {
            if let Some(files) = input.files() {
                let file_list = FileList::from(files);
                let files_vec: Vec<File> = file_list.iter().cloned().collect();
                set_selected_files.set(files_vec);
            }
        }
    };

    let on_submit = {
        let on_upload_success = on_upload_success.clone();
        move |ev: web_sys::SubmitEvent| {
            ev.prevent_default();
            let files = selected_files.get();
            if files.is_empty() {
                return;
            }
            
            let on_upload_success = on_upload_success.clone();
            spawn_local(async move {
                set_is_uploading.set(true);
                match upload_files(files).await {
                    Ok(response) => {
                        if response.success {
                            set_upload_message.set(Some((response.message, "success".to_string())));
                            set_selected_files.set(vec![]);
                            if let Some(input) = file_input_ref.get() {
                                input.set_value("");
                            }
                            on_upload_success();
                        } else {
                            set_upload_message.set(Some((response.message, "error".to_string())));
                        }
                    }
                    Err(e) => {
                        set_upload_message.set(Some((format!("upload failed: {}", e), "error".to_string())));
                    }
                }
                set_is_uploading.set(false);
                
                // Clear message after 5 seconds
                let set_upload_message_clone = set_upload_message;
                wasm_bindgen_futures::spawn_local(async move {
                    gloo_timers::future::sleep(std::time::Duration::from_secs(5)).await;
                    set_upload_message_clone.set(None);
                });
            });
        }
    };

    view! {
        <div class="upload-section">
            <form on:submit=on_submit>
                <div class="upload-controls">
                    <div class="file-input-wrapper">
                        <input 
                            type="file"
                            multiple=true
                            node_ref=file_input_ref
                            on:change=on_file_change
                            class="file-input"
                            accept="*/*"
                        />
                        <label class="file-input-button">"choose files"</label>
                    </div>
                    <button 
                        type="submit" 
                        class="upload-button"
                        disabled=move || selected_files.get().is_empty() || is_uploading.get()
                    >
                        {move || if is_uploading.get() { "uploading..." } else { "upload files" }}
                    </button>
                </div>
            </form>

            <Show when=move || !selected_files.get().is_empty()>
                <div class="selected-files">
                    <h4>"selected files:"</h4>
                    <div>
                        <For
                            each=move || selected_files.get()
                            key=|file| file.name().clone()
                            children=|file| {
                                view! {
                                    <div class="file-item">
                                        <span class="file-name">{file.name()}</span>
                                        <span class="file-size">{format_file_size(file.size() as u64)}</span>
                                    </div>
                                }
                            }
                        />
                    </div>
                </div>
            </Show>

            <Show when=move || upload_message.get().is_some()>
                {move || {
                    upload_message.get().map(|(message, msg_type)| {
                        view! {
                            <div class=format!("message {}", msg_type)>
                                {message}
                            </div>
                        }
                    })
                }}
            </Show>
        </div>
    }
}

#[component]
fn FilesSection<F>(
    files: Memo<Vec<FileInfo>>,
    search_term: ReadSignal<String>,
    set_search_term: WriteSignal<String>,
    is_loading: ReadSignal<bool>,
    on_delete: F,
) -> impl IntoView 
where 
    F: Fn(String) + Clone + 'static,
{
    view! {
        <div class="files-section">
            <div class="search-container">
                <input 
                    type="text"
                    class="search-input"
                    placeholder="search files... (use #image, #video, #audio, #text, #code, #pdf, #archive, #document to filter by type)"
                    prop:value=search_term
                    on:input=move |ev| {
                        let value = event_target_value(&ev);
                        set_search_term.set(value);
                    }
                />
                <div class="search-help">"tip: start with # to filter by file type (e.g., #image, #video)"</div>
            </div>

            <Show when=move || is_loading.get()>
                <div class="loading">
                    <div class="spinner"></div>
                    "loading files..."
                </div>
            </Show>

            <Show when=move || !is_loading.get()>
                <div class="files-grid">
                    <Show 
                        when=move || !files.get().is_empty()
                        fallback=move || {
                            let search = search_term.get().trim();
                            let (message, sub_message) = if search.is_empty() {
                                ("no files uploaded yet", "upload some files to get started!")
                            } else {
                                ("no files found matching search", "try a different search term or upload some files!")
                            };
                            view! {
                                <div class="empty-state">
                                    <h3>{message}</h3>
                                    <p>{sub_message}</p>
                                </div>
                            }
                        }
                    >
                        <For
                            each=move || files.get()
                            key=|file| file.path.clone()
                            children={
                                let on_delete = on_delete.clone();
                                move |file| {
                                    let on_delete = on_delete.clone();
                                    view! {
                                        <FileCard file=file on_delete=on_delete />
                                    }
                                }
                            }
                        />
                    </Show>
                </div>
            </Show>
        </div>
    }
}

#[component]
fn FileCard<F>(file: FileInfo, on_delete: F) -> impl IntoView 
where 
    F: Fn(String) + Clone + 'static,
{
    let (preview_content, set_preview_content) = create_signal::<Option<String>>(None);
    
    // Load preview if file can be previewed
    if file.can_preview {
        let file_path = file.path.clone();
        let file_type = file.file_type.clone();
        let file_name = file.name.clone();
        
        spawn_local(async move {
            if let Ok(content) = load_file_preview(&file_path, &file_name, &file_type).await {
                set_preview_content.set(Some(content));
            } else {
                set_preview_content.set(Some("preview not available".to_string()));
            }
        });
    }

    let delete_file = {
        let file_path = file.path.clone();
        move |_| {
            if web_sys::window()
                .and_then(|w| w.confirm_with_message("are you sure you want to delete this file?").ok())
                .unwrap_or(false)
            {
                on_delete(file_path.clone());
            }
        }
    };

    view! {
        <div class="file-card">
            <div class="file-header">
                <div class="file-info">
                    <div class="file-name">
                        <span class=format!("file-type-icon file-type-{}", file.file_type)>
                            {get_file_type_icon(&file.file_type)}
                        </span>
                        {&file.name}
                    </div>
                    <div class="file-size">{format_file_size(file.size)}</div>
                </div>
                <div class="file-actions">
                    <a 
                        href=format!("/download/{}", file.path)
                        class="btn btn-download"
                        download=true
                    >
                        "download"
                    </a>
                    <button 
                        class="btn btn-delete"
                        on:click=delete_file
                    >
                        "delete"
                    </button>
                </div>
            </div>
            
            <Show when=move || file.can_preview>
                <div class="file-preview">
                    {move || {
                        preview_content.get()
                            .map(|content| view! { <div inner_html=content></div> })
                            .unwrap_or_else(|| view! { <div>"loading preview..."</div> })
                    }}
                </div>
            </Show>
        </div>
    }
}

// Utility functions
async fn load_files_and_storage(
    set_files: WriteSignal<Vec<FileInfo>>,
    set_storage_info: WriteSignal<Option<StorageInfo>>,
    set_is_loading: WriteSignal<bool>,
) {
    set_is_loading.set(true);
    
    // Make requests concurrently
    let files_request = async {
        let response = Request::get("/files").send().await?;
        response.json::<FilesResponse>().await
    };
    
    let storage_request = async {
        let response = Request::get("/storage").send().await?;
        response.json::<StorageInfo>().await
    };
    
    // Wait for both requests to complete
    match (files_request.await, storage_request.await) {
        (Ok(files_data), Ok(storage_data)) => {
            set_files.set(files_data.files);
            set_storage_info.set(Some(storage_data));
        },
        (Ok(files_data), Err(_)) => {
            set_files.set(files_data.files);
        },
        (Err(_), Ok(storage_data)) => {
            set_storage_info.set(Some(storage_data));
        },
        (Err(_), Err(_)) => {
            // Both requests failed, keep current state
        }
    }
    
    set_is_loading.set(false);
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

async fn delete_file(filename: &str) -> Result<ApiResponse, String> {
    let response = Request::post(&format!("/delete/{}", filename))
        .send()
        .await
        .map_err(|e| format!("Request failed: {:?}", e))?;
        
    response.json::<ApiResponse>().await
        .map_err(|e| format!("Failed to parse response: {:?}", e))
}

async fn load_file_preview(file_path: &str, file_name: &str, file_type: &str) -> Result<String, String> {
    match file_type {
        "image" => Ok(format!(r#"<img src="/download/{}" alt="{}" />"#, file_path, file_name)),
        "video" => Ok(format!(
            r#"<video controls>
                <source src="/download/{}" type="video/mp4">
                <source src="/download/{}" type="video/webm">
                your browser does not support the video tag.
            </video>"#,
            file_path, file_path
        )),
        "audio" => Ok(format!(
            r#"<audio controls>
                <source src="/download/{}" type="audio/mpeg">
                <source src="/download/{}" type="audio/wav">
                your browser does not support the audio tag.
            </audio>"#,
            file_path, file_path
        )),
        "pdf" => Ok(format!(r#"<iframe src="/download/{}" class="file-preview-pdf"></iframe>"#, file_path)),
        "text" | "code" => {
            let response = Request::get(&format!("/preview/{}", file_path))
                .send()
                .await
                .map_err(|e| format!("Request failed: {:?}", e))?;
                
            let data: PreviewResponse = response.json().await
                .map_err(|e| format!("Failed to parse response: {:?}", e))?;
                
            match data.content {
                Some(content) => {
                    let truncated = if content.len() > 200 {
                        format!("{}...", &content[..200])
                    } else {
                        content
                    };
                    Ok(format!(r#"<div class="file-preview-text">{}</div>"#, html_escape(&truncated)))
                }
                None => Ok(format!(r#"<div class="file-preview-text">error loading preview: {}</div>"#, 
                    data.error.unwrap_or_else(|| "unknown error".to_string())))
            }
        }
        _ => Ok(r#"<div class="file-preview-text">preview not available</div>"#.to_string())
    }
}

fn format_file_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 bytes".to_string();
    }
    
    let k = 1024u64;
    let sizes = ["bytes", "kb", "mb", "gb"];
    let i = (bytes as f64).log(k as f64).floor() as usize;
    let size = bytes as f64 / (k.pow(i as u32) as f64);
    
    format!("{:.2} {}", size, sizes.get(i).unwrap_or(&"bytes"))
}

fn get_file_type_icon(file_type: &str) -> &'static str {
    match file_type {
        "image" => "🖼️",
        "video" => "🎥",
        "audio" => "🎵",
        "text" => "📄",
        "code" => "💻",
        "pdf" => "📕",
        "archive" => "📦",
        "document" => "📋",
        _ => "📄"
    }
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[wasm_bindgen]
pub fn run_app() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App /> });
}
