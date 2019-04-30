all: ./target/x86_64-unknown-linux-musl/release/ffmpeg-screen-recorder

./target/x86_64-unknown-linux-musl/release/ffmpeg-screen-recorder: $(shell find . -type f -iname '*.rs' -o -name 'Cargo.toml' | sed 's/ /\\ /g')
	cargo build --release --target x86_64-unknown-linux-musl
	strip ./target/x86_64-unknown-linux-musl/release/ffmpeg-screen-recorder
	
install:
	$(MAKE)
	sudo cp ./target/x86_64-unknown-linux-musl/release/ffmpeg-screen-recorder /usr/local/bin/ffmpeg-screen-recorder
	sudo chown root. /usr/local/bin/ffmpeg-screen-recorder
	sudo chmod 0755 /usr/local/bin/ffmpeg-screen-recorder

uninstall:
	sudo rm /usr/local/bin/ffmpeg-screen-recorder

test:
	cargo test --verbose
	
clean:
	cargo clean
