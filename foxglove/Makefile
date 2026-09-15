BINARY_NAME ?= foxglove
GOLANGCI_LINT_VERSION ?= v2.12.2
GO_BIN_DIR := $(shell go env GOBIN)
ifeq ($(GO_BIN_DIR),)
GO_BIN_DIR := $(shell go env GOPATH)/bin
endif
GOLANGCI_LINT_BIN ?= $(GO_BIN_DIR)/golangci-lint

.PHONY: lint test compat version build build-release install

ifeq ($(GOOS), windows)
	BINARY_NAME := $(addsuffix .exe, $(BINARY_NAME))
endif

lint:
	@installed=$$("$(GOLANGCI_LINT_BIN)" version --short 2>/dev/null || true); \
		expected="$(GOLANGCI_LINT_VERSION:v%=%)"; \
		if [ "$$installed" != "$$expected" ]; then \
			echo "Installing golangci-lint $(GOLANGCI_LINT_VERSION)"; \
			go install github.com/golangci/golangci-lint/v2/cmd/golangci-lint@$(GOLANGCI_LINT_VERSION); \
		fi
	"$(GOLANGCI_LINT_BIN)" run ./...

test:
	go test ./... -cover

compat:
	go test ./compat -count=1

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
