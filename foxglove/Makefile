BINARY_NAME ?= foxglove

ifeq ($(GOOS), windows)
	BINARY_NAME := $(addsuffix .exe, $(BINARY_NAME))
endif

lint:
	golangci-lint run ./...

test:
	go test ./... -cover

version: $(eval VERSION=$(shell git describe --tags))
version:
	@echo VERSION=$(VERSION)

build: version
	go build -tags 'netgo' -ldflags '-X main.Version=$(VERSION)'

build-release: version
	mkdir -p bin
	go build -tags 'netgo' -ldflags '-X main.Version=$(VERSION)' -o bin/$(BINARY_NAME)

install: version
	go install -tags 'netgo' -ldflags '-X main.Version=$(VERSION)'
