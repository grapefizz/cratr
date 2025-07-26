# Cratr - File Server

A simple and secure file upload/download server built with Actix Web and Leptos for Linux systems and home servers as cloud storage.

## Features

- **File Upload**: Upload multiple files with drag-and-drop support
- **File Download**: Download files with direct links  
- **File Management**: Delete files through the web interface
- **File Preview**: Preview images and videos directly in the browser
- **Search**: Search and filter files by name
- **Security**: Filename sanitization and file size limits
- **Responsive**: Mobile-friendly web interface with Catppuccin Mocha theme
- **Fast**: Built with Rust and Actix Web for high performance
- **Modern UI**: Grid-based layout with file type detection and storage info

## Installation

### Prerequisites

- Rust 1.70+ installed on your system
- Linux environment (tested on Arch, macOS, nixOS)

### Build and Run

1. Clone or download this project
2. Navigate to the project directory
3. Build and run the server:

```bash
cargo run --release
```

The server will start on `http://127.0.0.1:8080`

## Configuration

You can modify the following constants in `src/main.rs`:

- `MAX_FILE_SIZE`: Maximum file size (default: 256 MB)
- `MAX_FILE_COUNT`: Maximum files per upload (default: 3)
- `UPLOAD_DIR`: Directory to store uploaded files (default: ./uploads)

To change the server bind address, modify the `.bind()` call in the main function.

## API Endpoints

### Web Interface
- `GET /` - Main web interface (Leptos WASM frontend)

### File Operations
- `POST /upload` - Upload files (multipart/form-data)
- `GET /files` - List all uploaded files with metadata (JSON)
- `GET /download/{filename}` - Download a specific file
- `POST /delete/{filename}` - Delete a specific file
- `GET /storage` - Get storage usage information

### Example API Usage

Upload files:
```bash
curl -X POST -F "files=@example.txt" http://localhost:8080/upload
```

List files:
```bash
curl http://localhost:8080/files
```

Download file:
```bash
curl -O http://localhost:8080/download/{filename}
```

Delete file:
```bash
curl -X POST http://localhost:8080/delete/{filename}
```

Get storage info:
```bash
curl http://localhost:8080/storage
```

## Security Features

- Filename sanitization to prevent path traversal attacks
- UUID prefixes to prevent filename conflicts
- File size limits to prevent disk space exhaustion
- File count limits per upload request
- CORS protection for API endpoints

## Architecture

This project uses a modern Rust web stack:

- **Backend**: Actix Web server with REST API
- **Frontend**: Leptos framework compiled to WebAssembly
- **Styling**: CSS-in-Rust with Catppuccin Mocha color scheme
- **Build**: Trunk for WASM compilation and asset bundling

## Directory Structure

```
cratr/
├── src/
│   ├── main.rs          # Backend server and API routes
│   ├── lib.rs           # Shared data structures
│   └── frontend.rs      # Leptos frontend (WASM)
├── static/              # Built frontend assets and index.html
│   ├── index.html       # Main HTML file
│   ├── cratr.js         # Generated WASM bindings
│   └── cratr_bg.wasm    # Compiled WebAssembly
├── uploads/             # Uploaded files (created automatically)
├── pkg/                 # wasm-pack output directory
├── build_wasm.sh        # Build script for frontend
├── Cargo.toml           # Dependencies
└── README.md            # This file
```

## Development

### Frontend Development

The frontend is built with Leptos and compiled to WebAssembly. To work on the frontend:

1. Build the frontend:
```bash
./build_wasm.sh
```

Or manually with wasm-pack:
```bash
wasm-pack build --target web --out-dir pkg
```

The build process generates WebAssembly files in the `static/` directory that are served by the Actix Web server.

### Features

- **Grid Layout**: Responsive grid-based file display
- **File Previews**: Automatic preview generation for images and videos
- **Search**: Real-time search with hash-based filtering
- **Storage Info**: Display available disk space and usage
- **File Type Detection**: Visual file type indicators with color coding
- **Drag & Drop**: Modern file upload interface

## Production Deployment

For production use:

1. **Build**: Create optimized builds for both frontend and backend
2. **Reverse Proxy**: Use nginx or Apache as a reverse proxy
3. **SSL/TLS**: Enable HTTPS for secure file transfers
4. **Firewall**: Configure firewall rules appropriately
5. **File Limits**: Adjust file size and count limits based on your needs
6. **Backup**: Implement regular backups of the uploads directory
7. **Monitoring**: Set up logging and monitoring

Example nginx configuration:
```nginx
server {
    listen 80;
    server_name your-domain.com;
    
    client_max_body_size 256M;
    
    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

## License

This project is open source and available under the MIT License.
