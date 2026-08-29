.PHONY: lint test compat build install

lint:
	$(MAKE) -C foxglove lint

test:
	$(MAKE) -C foxglove test

compat:
	$(MAKE) -C foxglove compat

build:
	$(MAKE) -C foxglove build

install:
	$(MAKE) -C foxglove install
