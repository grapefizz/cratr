# Cratr - File Server

A simple and secure file upload/download server built with Actix Web for Linux systems and home servers as cloud storage.

## Features

- 📤 **File Upload**: Upload multiple files with drag-and-drop support
- 📥 **File Download**: Download files with direct links
- 🗑️ **File Management**: Delete files through the web interface
- 🔒 **Security**: Filename sanitization and file size limits
- 📱 **Responsive**: Mobile-friendly web interface
- ⚡ **Fast**: Built with Rust and Actix Web for high performance

## Installation

### Prerequisites

- Rust 1.70+ installed on your system
- Linux environment (tested on Ubuntu, CentOS, Debian)

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
- `GET /` - Main web interface

### File Operations
- `POST /upload` - Upload files (multipart/form-data)
- `GET /files` - List all uploaded files (JSON)
- `GET /download/{filename}` - Download a specific file
- `POST /delete/{filename}` - Delete a specific file

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

## Security Features

- Filename sanitization to prevent path traversal attacks
- UUID prefixes to prevent filename conflicts
- File size limits to prevent disk space exhaustion
- File count limits per upload request

## Directory Structure

```
cratr/
├── src/
│   └── main.rs          # Main application code
├── static/
│   └── index.html       # Web interface
├── uploads/             # Uploaded files (created automatically)
├── Cargo.toml           # Dependencies
└── README.md            # This file
```

## Production Deployment

For production use:

1. **Reverse Proxy**: Use nginx or Apache as a reverse proxy
2. **SSL/TLS**: Enable HTTPS for secure file transfers
3. **Firewall**: Configure firewall rules appropriately
4. **File Limits**: Adjust file size and count limits based on your needs
5. **Backup**: Implement regular backups of the uploads directory
6. **Monitoring**: Set up logging and monitoring

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
    }
}
```

## License

This project is open source and available under the MIT License.
