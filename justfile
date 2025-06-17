set windows-shell := ["powershell.exe"]

# https://docs.rs/env_logger/latest/env_logger/#enabling-logging
# export RUST_LOG := "TRACE"
export RUST_BACKTRACE := "1"
# export GAME_LAUNCHED_FROM_EDITOR := "0"

@just:
  just --list

alias b := build
build: check
  cargo build --frozen

release: clean lint check
  cargo build --frozen --release

check:
  cargo check --all --tests
  cargo fmt --all --check

clean:
  cargo clean

alias d := doc
doc:
  cargo doc

format:
  cargo fmt --all

fix:
  cargo clippy --fix

lint:
  cargo clippy -- -D warnings

alias rb:= rebuild
rebuild: clean build doc

alias u := update
update:
  cargo update
  cargo clean
  cargo build
  cargo doc

alias e := godot-editor
godot-editor:
  fish -c "cd (findup .jj || findup .git) && cd ../godot && godot-45dev5 -e -w scenes/main.tscn &"

alias r := run
run: build
  # can append --position 2160,720 to explicitly specify launch position
  fish -c "cd (findup .jj || findup .git) && cd ../godot && godot-45dev5 -w scenes/main.tscn"

test:
  cargo test --all -- --nocapture

alias w := watch
watch:
  watchexec --exts rs,toml "clear; just build"

@versions:
  rustc --version
  cargo fmt -- --version
  cargo clippy -- --version
