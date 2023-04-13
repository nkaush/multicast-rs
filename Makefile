ifeq (, $(shell which cargo))
$(warning No `cargo` in path, consider installing Cargo with:)
$(warning - `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
$(warning - Or visit https://www.rust-lang.org/tools/install for more on installing Rust.)
$(error Unable to invoke cargo)
endif

.PHONY: fault-tolerant-atm clean

fault-tolerant-atm:
	cargo build --release
	cp target/release/fault-tolerant-atm ./ft-atm

clean:
	cargo clean
	rm -f fault-tolerant-atm Cargo.lock