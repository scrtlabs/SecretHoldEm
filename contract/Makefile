.PHONY: all build clean store

all: build

build:
	RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --locked
	wasm-opt -Os ./target/wasm32-unknown-unknown/release/*.wasm -o ./contract.wasm
	cat ./contract.wasm | gzip -9 > ./contract.wasm.gz 

clean:
	cargo clean
	-rm -f ./contract.wasm ./contract.wasm.gz

store: all
	secretcli tx compute store contract.wasm.gz --from mykey -y --gas 10000000 --source "https://github.com/enigmampc/SecretHoldEm/blob/$(shell git show --oneline -s | cut -f 1 -d ' ')/contract/src/contract.rs" -b block