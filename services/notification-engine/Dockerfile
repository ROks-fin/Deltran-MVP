FROM golang:1.21-alpine AS builder

RUN apk add --no-cache git make ca-certificates

WORKDIR /app

COPY go.mod go.sum ./
RUN go mod download

COPY . .

RUN CGO_ENABLED=0 GOOS=linux go build -a -installsuffix cgo -o notification-engine cmd/server/main.go

FROM alpine:latest

RUN apk --no-cache add ca-certificates tzdata

WORKDIR /root/

COPY --from=builder /app/notification-engine .
COPY --from=builder /app/config.yaml .
COPY --from=builder /app/templates ./templates

EXPOSE 8085 8086

CMD ["./notification-engine"]
