lint:
	golangci-lint run ./...

test:
	go test ./... -cover

gitcommit: $(eval GIT_COMMIT=$(shell git log -1 --pretty=format:"%H"))
gitcommit:
	@echo GIT_COMMIT=$(GIT_COMMIT)

build: gitcommit
	go build -ldflags "-X main.GitCommit=$(GIT_COMMIT)"

install: gitcommit
	go install -ldflags "-X main.GitCommit=$(GIT_COMMIT)"
