ifeq (, $(shell which cargo))
$(warning No `cargo` in path, consider installing Cargo with:)
$(warning - `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
$(warning - Or visit https://www.rust-lang.org/tools/install for more on installing Rust.)
$(error Unable to invoke cargo)
endif

.PHONY: mp1_node 

mp1_node:
	cargo build --release
	cp target/release/mp1_node ./mp1_node

clean:
	cargo clean
	rm -f mp1_node metrics.json