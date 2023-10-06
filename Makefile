#!/bin/bash

all:
	SLINT_STYLE=fluent cargo build --release

build:
	SLINT_STYLE=fluent cargo build --release

build-timings:
	SLINT_STYLE=fluent cargo build --release --timings
	cp -rf ./target/cargo-timings/cargo-timing.html ./profile

build-debug:
	SLINT_STYLE=fluent cargo build

run:
	SLINT_STYLE=fluent RUST_LOG=error,warn,info,debug,reqwest=on cargo run

run-local:
	RUST_LOG=error,warn,info,debug,reqwest=on ./target/debug/gossipbox

run-local-release:
	RUST_LOG=error,warn,info,debug,libp2p=off,multistream-select=off ./target/release/gossipbox

clippy:
	cargo clippy

clean-incremental:
	rm -rf ./target/debug/incremental/*

clean:
	cargo clean

install:
	cp -rf ./target/release/gossipbox ~/bin/

pack-win:
	./pack-win-package.sh

slint-view:
	slint-viewer --style fluent --auto-reload -I gossipbox/ui ./gossipbox/ui/appwindow.slint
