serve: build
	miniserve --index index.html ./target/wasm32/
build: src Cargo.toml static index.html
	wasm-pack build -d target/wasm32/pkg --target web -- --features web
	cp index.html target/wasm32
	cp -r static target/wasm32