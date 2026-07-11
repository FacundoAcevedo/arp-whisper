SHELL := /bin/sh

CARGO ?= cargo
CONFIG ?= example-config/config.ini

.DEFAULT_GOAL := help

.PHONY: help
help:
	@printf '%s\n' 'Available targets:' \
		'  help        Show this help' \
		'  fmt         Format the code with rustfmt' \
		'  fmt-check   Check formatting without changing files' \
		'  check       Type-check the project without building artifacts' \
		'  clippy      Run clippy across all targets' \
		'  test        Run the test suite' \
		'  coverage    Run test coverage with tarpaulin' \
		'  build       Build the binary in debug mode' \
		'  build-release Build the binary in release mode' \
		'  run         Run the binary with CONFIG=$(CONFIG)' \
		'  clean       Remove build artifacts' \
		'  install     Install the binary via cargo' \
		'  validate    Run formatting, check, clippy, and tests' \
		'  validations  Alias for validate'

.PHONY: fmt
fmt:
	$(CARGO) fmt

.PHONY: fmt-check
fmt-check:
	$(CARGO) fmt -- --check

.PHONY: check
check:
	$(CARGO) check --all-targets

.PHONY: clippy
clippy:
	$(CARGO) clippy --all-targets --all-features -- -D warnings

.PHONY: lint
lint: clippy

.PHONY: test
test:
	$(CARGO) test --all-targets

.PHONY: coverage
coverage:
	$(CARGO) tarpaulin --all-features --all-targets --out Html --output-dir target/coverage

.PHONY: build
build:
	$(CARGO) build

.PHONY: build-release
build-release:
	$(CARGO) build --release

.PHONY: run
run:
	$(CARGO) run -- $(CONFIG)

.PHONY: clean
clean:
	$(CARGO) clean

.PHONY: install
install:
	$(CARGO) install --path .

.PHONY: validate
validate: fmt-check check clippy test

.PHONY: validations
validations: validate
