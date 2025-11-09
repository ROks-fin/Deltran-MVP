.PHONY: build run test clean docker-build docker-run migrate

build:
	go build -o bin/notification-engine cmd/server/main.go

run:
	go run cmd/server/main.go

test:
	go test -v -race -coverprofile=coverage.out ./...

coverage:
	go tool cover -html=coverage.out

clean:
	rm -rf bin/ coverage.out

docker-build:
	docker build -t notification-engine:latest .

docker-run:
	docker run -p 8085:8085 -p 8086:8086 --env-file .env notification-engine:latest

migrate:
	psql -U deltran -d deltran -f migrations/001_init.sql

lint:
	golangci-lint run

deps:
	go mod download
	go mod tidy

.DEFAULT_GOAL := build
