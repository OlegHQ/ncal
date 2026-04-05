BINARY   := ncal
CRATE    := ncal-cli
PREFIX   ?= $(HOME)/.local
BINDIR   ?= $(PREFIX)/bin

# Release build flags — static link via rustls (no openssl), strip symbols.
CARGO_FLAGS := --release --package $(CRATE)

# macOS + Nix: the linker needs libiconv from the SDK.
ifeq ($(shell uname),Darwin)
  export LIBRARY_PATH := $(shell xcrun --show-sdk-path 2>/dev/null)/usr/lib
endif

.PHONY: build install uninstall check lint fmt test clean

build:
	cargo build $(CARGO_FLAGS)

install: build
	install -d $(BINDIR)
	install -m 755 target/release/$(BINARY) $(BINDIR)/$(BINARY)
	@echo "installed $(BINDIR)/$(BINARY)"

uninstall:
	rm -f $(BINDIR)/$(BINARY)
	@echo "removed $(BINDIR)/$(BINARY)"

check:
	cargo check --workspace

lint:
	cargo clippy --workspace -- -D warnings

fmt:
	cargo fmt --all -- --check

test:
	cargo test --workspace

clean:
	cargo clean
