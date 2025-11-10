#!/bin/bash

# Применение эталонного Dockerfile ко всем Go сервисам
# Использует GOTOOLCHAIN=auto для автоматического разрешения версий

echo "🔧 Применение эталонного Dockerfile ко всем Go сервисам..."
echo ""

# Массив Go сервисов
GO_SERVICES=("gateway" "reporting-engine" "notification-engine")

for service in "${GO_SERVICES[@]}"; do
    echo "📦 Обрабатываем: $service"

    SERVICE_DIR="services/$service"

    if [ ! -d "$SERVICE_DIR" ]; then
        echo "  ⚠️  Директория $SERVICE_DIR не найдена, пропускаем..."
        continue
    fi

    cd "$SERVICE_DIR"

    # Сохраняем старый Dockerfile как backup
    if [ -f Dockerfile ]; then
        cp Dockerfile Dockerfile.backup
        echo "  💾 Создан backup: Dockerfile.backup"
    fi

    # Создаем новый Dockerfile на основе шаблона
    cat > Dockerfile << 'DOCKERFILE_END'
# syntax=docker/dockerfile:1

# ============================================
# Stage 1: Builder
# ============================================
FROM golang:1.23-alpine AS builder

RUN apk add --no-cache git ca-certificates tzdata

WORKDIR /build

# Копируем go.mod и go.sum для кеширования зависимостей
COPY go.mod go.sum ./

# Загрузка зависимостей с автоматическим разрешением версий
# GOTOOLCHAIN=auto позволяет Go автоматически скачать нужную версию
RUN go env -w GOTOOLCHAIN=auto && \
    go mod download && \
    go mod verify

# Копируем исходный код
COPY . .

# Компиляция (определяем входной файл автоматически)
RUN if [ -f "cmd/main.go" ]; then \
        CGO_ENABLED=0 GOOS=linux GOARCH=amd64 \
        go build -ldflags="-s -w" -a -installsuffix cgo \
        -o /app/service ./cmd/main.go; \
    elif [ -f "main.go" ]; then \
        CGO_ENABLED=0 GOOS=linux GOARCH=amd64 \
        go build -ldflags="-s -w" -a -installsuffix cgo \
        -o /app/service ./main.go; \
    else \
        CGO_ENABLED=0 GOOS=linux GOARCH=amd64 \
        go build -ldflags="-s -w" -a -installsuffix cgo \
        -o /app/service .; \
    fi

# ============================================
# Stage 2: Runtime
# ============================================
FROM alpine:3.21 AS runtime

RUN apk add --no-cache ca-certificates tzdata && \
    addgroup -g 1000 appuser && \
    adduser -D -u 1000 -G appuser appuser

WORKDIR /app

COPY --from=builder --chown=appuser:appuser /app/service /app/service

USER appuser

EXPOSE 8080

ENTRYPOINT ["/app/service"]
DOCKERFILE_END

    echo "  ✅ Dockerfile обновлен с GOTOOLCHAIN=auto"

    cd ../..
done

echo ""
echo "✅ Все Go сервисы обновлены!"
echo ""
echo "📋 Обновленные сервисы:"
for service in "${GO_SERVICES[@]}"; do
    echo "  - $service"
done
echo ""
echo "🔧 Теперь Go автоматически скачает нужную версию компилятора при сборке"
echo ""
echo "📝 Команда для сборки:"
echo "   docker-compose build gateway reporting-engine notification-engine"
