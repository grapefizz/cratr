.PHONY: build-wasm build-server dev run clean

# Build the WASM frontend
build-wasm:
	./build_wasm.sh

# Build the server binary
build-server:
	cargo build --bin cratr

# Build everything
build: build-wasm build-server

# Development build (debug mode)
dev: build-wasm
	cargo build --bin cratr

# Run the server (will rebuild if needed)
run: build-wasm
	cargo run --bin cratr

# Clean build artifacts
clean:
	cargo clean
	rm -rf pkg/
	rm -f static/cratr.js static/cratr_bg.wasm
