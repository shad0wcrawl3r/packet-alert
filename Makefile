.ONESHELL:
.PHONY: Cargo.toml

build:
	cargo build
	cp target/debug/packet-alert ./packet-alert


test:
	cargo build
	sudo ./target/debug/packet-alert
# release:
# 	cargo build --release
# 	cp target/release/packet-alert ./packet-alert
	

clean: 
	rm -rf target
