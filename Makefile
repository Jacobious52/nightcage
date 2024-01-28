web:
	cargo build --release --target wasm32-unknown-unknown --features bevy/webgl2
	wasm-bindgen --no-typescript --target web \
		--out-dir ./out/ \
		--out-name "nightcage" \
		./target/wasm32-unknown-unknown/release/nightcage.wasm
	wasm-opt -Oz -o ./out/nightcage_bg.wasm ./out/nightcage_bg.wasm
	