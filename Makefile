default: build

all: test

test: build
	cargo test

build:
	stellar contract build
	@ls -l target/wasm32v1-none/release/*.wasm
	@$(MAKE) check-size

check-size:
	@MAX_SIZE=262144; \
	for wasm in target/wasm32v1-none/release/*.wasm; do \
		size=$$(wc -c < "$$wasm"); \
		if [ "$$size" -gt "$$MAX_SIZE" ]; then \
			echo "$$wasm is too large: $$size bytes (limit: $$MAX_SIZE)"; \
			exit 1; \
		fi; \
	done

fmt:
	cargo fmt --all

clean:
	cargo clean
