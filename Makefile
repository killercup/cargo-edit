FEATURES ?= #--features "dev"

build:
	cargo build $(FEATURES)

release:
	cargo build --release $(FEATURES)

fmt:
	rustfmt src/lib.rs --write-mode=overwrite && \
	rustfmt src/bin/list/main.rs --write-mode=overwrite && \
	rustfmt src/bin/add/main.rs --write-mode=overwrite && \
	rustfmt src/bin/rm/main.rs --write-mode=overwrite && \
	find tests/*.rs -exec rustfmt "{}" --write-mode=overwrite ";"

.PHONY: build release fmt
