FEATURES ?= --feature "dev"

build:
	cargo build $(FEATURES)

release:
	cargo build --release $(FEATURES)

fmt:
	rustfmt src/bin/list/main.rs --write-mode=overwrite && \
	rustfmt src/bin/add/main.rs --write-mode=overwrite && \
	rustfmt tests/*.rs --write-mode=overwrite

.PHONY: build release fmt
