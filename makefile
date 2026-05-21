BIN = rustblocks
DEST = /usr/local/bin/$(BIN)

all: build install

build:
	cargo build --release

install:
	sudo cp target/release/$(BIN) $(DEST)

clean:
	cargo clean

.PHONY: all build install clean
