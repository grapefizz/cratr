use leptos::*;
use wasm_bindgen::prelude::*;
use gloo_net::http::Request;
use gloo_file::{FileList, File};
use web_sys::{Event, FormData};

use crate::{FileInfo, FilesResponse, StorageInfo, ApiResponse, PreviewResponse};

#[component]
pub fn App() -> impl IntoView {
    let (files, set_files) = create_signal(Vec::<FileInfo>::new());
    let (storage_info, set_storage_info) = create_signal(None::<StorageInfo>);
    let (search_term, set_search_term) = create_signal(String::new());
    let (is_loading, set_is_loading) = create_signal(false);

    // Load files when component mounts
    create_effect(move |_| {
        spawn_local(async move {
            load_files_and_storage(set_files, set_storage_info, set_is_loading).await;
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
            <StorageSection storage_info=storage_info />
            <UploadSection 
                on_upload_complete=move || {
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
                set_files=set_files
                set_storage_info=set_storage_info
                set_is_loading=set_is_loading
            />
        </div>
    }
}

#[component]
pub fn StorageSection(
    storage_info: ReadSignal<Option<StorageInfo>>,
) -> impl IntoView {
    view! {
        <div class="storage-info">
            <Show when=move || storage_info.get().is_some() fallback=|| view! { <div>"Loading storage info..."</div> }>
                {move || {
                    if let Some(info) = storage_info.get() {
                        view! {
                            <p>
                                "storage: " {info.formatted_disk_free} " free of " {info.formatted_disk_total} 
                                " (" {format!("{:.1}%", info.disk_used_percentage)} " used)"
                            </p>
                        }.into_view()
                    } else {
                        view! { <p></p> }.into_view()
                    }
                }}
            </Show>
        </div>
    }
}

#[component]
pub fn UploadSection<F>(
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
        <div class="upload-section">
            <form on:submit=on_submit>
                <div class="file-input-container">
                    <input
                        type="file"
                        id="fileInput"
                        multiple
                        ref=file_input_ref
                        on:change=on_file_change
                        class="file-input"
                        accept="*/*"
                    />
                    <label for="fileInput" class="file-input-label">
                        "choose files to upload"
                    </label>
                </div>
                
                <Show when=move || !selected_files.get().is_empty()>
                    <div class="selected-files">
                        <h4>"selected files:"</h4>
                        <ul>
                            <For
                                each=move || selected_files.get()
                                key=|file| file.name()
                                let:file
                            >
                                <li>{file.name()}</li>
                            </For>
                        </ul>
                    </div>
                </Show>
                
                <div style="margin: 10px 0; color: #565f89; font-size: 0.8em;">
                    "Debug: Files selected: " {move || selected_files.get().len()} 
                    " | Uploading: " {move || if is_uploading.get() { "Yes" } else { "No" }}
                    " | Button disabled: " {move || if selected_files.get().is_empty() || is_uploading.get() { "Yes" } else { "No" }}
                </div>
                
                <button 
                    type="button"
                    disabled=move || selected_files.get().is_empty() || is_uploading.get()
                    class="upload-button"
                    on:click=on_upload_click
                >
                    {move || if is_uploading.get() { "uploading..." } else { "upload files" }}
                </button>
            </form>
        </div>
    }
}

#[component]
fn FilesSection(
    files: Memo<Vec<FileInfo>>,
    search_term: ReadSignal<String>,
    set_search_term: WriteSignal<String>,
    is_loading: ReadSignal<bool>,
    set_files: WriteSignal<Vec<FileInfo>>,
    set_storage_info: WriteSignal<Option<StorageInfo>>,
    set_is_loading: WriteSignal<bool>,
) -> impl IntoView 
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
            
            <Show 
                when=move || is_loading.get()
                fallback=move || {
                    view! {
                        <div class="files-grid">
                            <Show
                                when=move || !files.get().is_empty()
                                fallback=move || {
                                    let search_value = search_term.get();
                                    let search = search_value.trim();
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
                                    let:file
                                >
                                    <FileCard 
                                        file=file
                                        set_files=set_files
                                        set_storage_info=set_storage_info
                                        set_is_loading=set_is_loading
                                    />
                                </For>
                            </Show>
                        </div>
                    }
                }
            >
                <div class="loading">"loading files..."</div>
            </Show>
        </div>
    }
}

#[component]
fn FileCard(
    file: FileInfo,
    set_files: WriteSignal<Vec<FileInfo>>,
    set_storage_info: WriteSignal<Option<StorageInfo>>,
    set_is_loading: WriteSignal<bool>,
) -> impl IntoView 
{
    let (preview_content, set_preview_content) = create_signal(None::<String>);
    let (show_preview, set_show_preview) = create_signal(false);

    let on_preview = {
        let file_path = file.path.clone();
        let file_name = file.name.clone();
        move |_| {
            if file.can_preview {
                let file_path = file_path.clone();
                spawn_local(async move {
                    let response = Request::get(&format!("/preview/{}", &file_path))
                        .send()
                        .await;
                    
                    if let Ok(resp) = response {
                        if let Ok(preview) = resp.json::<PreviewResponse>().await {
                            if let Some(content) = preview.content {
                                set_preview_content.set(Some(content));
                                set_show_preview.set(true);
                            } else {
                                set_preview_content.set(Some("preview not available".to_string()));
                                set_show_preview.set(true);
                            }
                        }
                    }
                });
            } else {
                set_preview_content.set(Some("preview not available".to_string()));
                set_show_preview.set(true);
            }
        }
    };

    let delete_file = {
        let file_path = file.path.clone();
        move |_| {
            if web_sys::window()
                .and_then(|w| w.confirm_with_message("are you sure you want to delete this file?").ok())
                .unwrap_or(false)
            {
                let file_path = file_path.clone();
                spawn_local(async move {
                    if let Ok(_) = delete_file_api(&file_path).await {
                        load_files_and_storage(set_files, set_storage_info, set_is_loading).await;
                    }
                });
            }
        }
    };

    view! {
        <div class="file-card">
            <div class="file-icon" data-type=&file.file_type></div>
            <div class="file-info">
                <div class="file-name" title=&file.name>{&file.name}</div>
                <div class="file-meta">
                    <span class="file-size">{format_file_size(file.size)}</span>
                    <span class="file-type">{&file.file_type}</span>
                </div>
            </div>
            <div class="file-actions">
                <button 
                    class="action-button preview-button" 
                    on:click=on_preview
                    disabled=move || !file.can_preview
                >
                    "👁"
                </button>
                <a href=&format!("/download/{}", &file.path) download=&file.name class="action-button download-button">
                    "⬇"
                </a>
                <button class="action-button delete-button" on:click=delete_file>
                    "🗑"
                </button>
            </div>
            
            <Show when=move || show_preview.get()>
                <div class="preview-modal" on:click=move |_| set_show_preview.set(false)>
                    <div class="preview-content" on:click=|e| e.stop_propagation()>
                        <div class="preview-header">
                            <h3>{&file.name}</h3>
                            <button class="close-button" on:click=move |_| set_show_preview.set(false)>
                                "×"
                            </button>
                        </div>
                        <div class="preview-body">
                            {move || {
                                if let Some(content) = preview_content.get() {
                                    view! { <pre>{content}</pre> }.into_view()
                                } else {
                                    view! { <pre>"loading..."</pre> }.into_view()
                                }
                            }}
                        </div>
                    </div>
                </div>
            </Show>
        </div>
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

#[wasm_bindgen]
pub fn run() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App /> });
}
