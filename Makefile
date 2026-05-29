VERSION ?= 0.1.0

.PHONY: run stop verify package check swift-build rust-check

run:
	./script/build_and_run.sh

stop:
	./script/build_and_run.sh stop

verify:
	./script/build_and_run.sh --verify

package:
	./script/build_and_run.sh --package

check: rust-check swift-build

rust-check:
	cargo check --manifest-path crates/proxyman-core/Cargo.toml --bin proxyman-sidecar

swift-build:
	swift build --package-path swiftui
