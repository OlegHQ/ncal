BINARY         := ncal
CRATE          := ncal
CARGO          ?= cargo
PREFIX         ?= $(HOME)/.local
BINDIR         ?= $(PREFIX)/bin
RELEASE_BRANCH ?= dev

# Release build flags: rustls, stripped symbols, LTO.
CARGO_FLAGS := --release --package $(CRATE)

# macOS + Nix: the linker needs libiconv from the SDK.
ifeq ($(shell uname),Darwin)
  export LIBRARY_PATH := $(shell xcrun --show-sdk-path 2>/dev/null)/usr/lib
endif

.PHONY: help all build install uninstall check lint fmt fmt-check fix test ci hooks clean release-local dist-plan minor patch ship-minor ship-patch check-clean _ship

.DEFAULT_GOAL := help

help:
	@echo "Local build / install:"
	@echo "  build, all       release build"
	@echo "  install          install to BINDIR ($(BINDIR))"
	@echo "  uninstall        remove $(BINDIR)/$(BINARY)"
	@echo ""
	@echo "Quality gates:"
	@echo "  fmt              run rustfmt"
	@echo "  fix              rustfmt + cargo clippy --fix"
	@echo "  ci               fmt-check + clippy + tests"
	@echo "  hooks            install tracked git hooks"
	@echo ""
	@echo "Release:"
	@echo "  dist-plan        validate cargo-dist release plan"
	@echo "  ship-patch       bump patch, commit, tag, push branch + tag"
	@echo "  ship-minor       bump minor, commit, tag, push branch + tag"

all: build

build:
	$(CARGO) build $(CARGO_FLAGS)

install: build
	install -d $(BINDIR)
	install -m 755 target/release/$(BINARY) $(BINDIR)/$(BINARY)
	@echo "installed $(BINDIR)/$(BINARY)"

uninstall:
	rm -f $(BINDIR)/$(BINARY)
	@echo "removed $(BINDIR)/$(BINARY)"

check:
	$(CARGO) check --workspace

lint:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

fmt:
	$(CARGO) fmt --all

fmt-check:
	$(CARGO) fmt --all -- --check

fix:
	$(CARGO) fmt --all
	$(CARGO) clippy --workspace --all-targets --fix --allow-dirty --allow-staged -- -D warnings

test:
	$(CARGO) test --workspace

ci: fmt-check lint test

hooks:
	git config core.hooksPath .githooks
	@echo "git hooks installed (core.hooksPath -> .githooks)"

release-local:
	$(CARGO) build $(CARGO_FLAGS)

dist-plan:
	dist plan

minor:
	$(CARGO) run --package xtask -- bump-version minor

patch:
	$(CARGO) run --package xtask -- bump-version patch

check-clean:
	@git diff-index --quiet HEAD -- || (echo "error: dirty working tree (commit or stash first)"; exit 1)

ship-minor: check-clean
	$(CARGO) run --package xtask -- bump-version minor
	@$(MAKE) _ship

ship-patch: check-clean
	$(CARGO) run --package xtask -- bump-version patch
	@$(MAKE) _ship

_ship:
	@set -e; \
	v=$$($(CARGO) run --quiet --package xtask -- read-version); \
	$(CARGO) build >/dev/null 2>&1; \
	git add Cargo.toml Cargo.lock README.md; \
	git commit -m "chore(release): v$$v"; \
	git tag -a "v$$v" -m "v$$v"; \
	git push origin "$(RELEASE_BRANCH)"; \
	git push origin "v$$v"; \
	echo "Pushed v$$v; cargo-dist will publish GitHub release assets and the Homebrew formula."

clean:
	$(CARGO) clean
