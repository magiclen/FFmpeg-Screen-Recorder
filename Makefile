all: ./target/release/ffmpeg-screen-recorder

./target/release/ffmpeg-screen-recorder: ./src/lib.rs ./src/main.rs
	cargo build --release
	strip ./target/release/ffmpeg-screen-recorder
	
install:
	$(MAKE)
	sudo cp ./target/release/ffmpeg-screen-recorder /usr/local/bin/ffmpeg-screen-recorder
	sudo chown root. /usr/local/bin/ffmpeg-screen-recorder
	sudo chmod 0755 /usr/local/bin/ffmpeg-screen-recorder
	
clean:
	cargo clean
