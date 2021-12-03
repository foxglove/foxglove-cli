BINARY_NAME ?= foxglove

lint:
	golangci-lint run ./...

test:
	go test ./... -cover

version: $(eval VERSION=$(shell git describe --tags))
version:
	@echo VERSION=$(VERSION)

build: version
	go build -ldflags "-X main.Version=$(VERSION)"

build-release: version
	go build -ldflags "-X main.Version=$(VERSION)" -o $(BINARY_NAME)

install: version
	go install -ldflags "-X main.Version=$(VERSION)"
