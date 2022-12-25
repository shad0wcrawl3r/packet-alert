.ONESHELL:
.PHONY: Cargo.toml

build:
	cargo build
	cp target/debug/packet-alert ./packet-alert


test:
	cargo build
	sudo ./target/debug/packet-alert

debug:
	cargo build
	GDB_PATH = $(shell which rust-gdb)
	sudo lldb ./target/debug/packet-alert
# release:
# 	cargo build --release
# 	cp target/release/packet-alert ./packet-alert
	

clean: 
	rm -rf target
