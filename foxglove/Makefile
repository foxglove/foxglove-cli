BINARY_NAME ?= foxglove
GOLANGCI_LINT_VERSION ?= v2.12.2

ifeq ($(GOOS), windows)
	BINARY_NAME := $(addsuffix .exe, $(BINARY_NAME))
endif

lint:
	@installed=$$(golangci-lint version --short 2>/dev/null || true); \
	expected="$(GOLANGCI_LINT_VERSION:v%=%)"; \
	if [ "$$installed" != "$$expected" ]; then \
		echo "Installing golangci-lint $(GOLANGCI_LINT_VERSION)"; \
		go install github.com/golangci/golangci-lint/v2/cmd/golangci-lint@$(GOLANGCI_LINT_VERSION); \
	fi
	golangci-lint run ./...

test:
	go test ./... -cover

version: $(eval VERSION=$(shell git describe --tags 2>/dev/null || echo dev))
version:
	@echo VERSION=$(VERSION)

build: version
	go build -tags 'netgo' -ldflags '-X main.Version=$(VERSION)'

build-release: version
	mkdir -p bin
	go build -tags 'netgo' -ldflags '-X main.Version=$(VERSION)' -o bin/$(BINARY_NAME)

install: version
	go install -tags 'netgo' -ldflags '-X main.Version=$(VERSION)'
