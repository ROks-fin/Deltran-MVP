package integration

import (
	"context"
	"testing"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// TestClearingEngineGRPC tests gRPC connectivity to clearing engine
func TestClearingEngineGRPC(t *testing.T) {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	conn, err := grpc.DialContext(ctx, "localhost:50055",
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		t.Logf("Warning: Cannot connect to clearing engine gRPC: %v", err)
		t.Skip("Clearing engine gRPC not available")
		return
	}
	defer conn.Close()

	t.Log("✓ Successfully connected to clearing engine gRPC")
}

// TestSettlementEngineGRPC tests gRPC connectivity to settlement engine
func TestSettlementEngineGRPC(t *testing.T) {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	conn, err := grpc.DialContext(ctx, "localhost:50056",
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		t.Logf("Warning: Cannot connect to settlement engine gRPC: %v", err)
		t.Skip("Settlement engine gRPC not available")
		return
	}
	defer conn.Close()

	t.Log("✓ Successfully connected to settlement engine gRPC")
}

// TestGRPCLatency tests gRPC call latency
func TestGRPCLatency(t *testing.T) {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	start := time.Now()

	conn, err := grpc.DialContext(ctx, "localhost:50055",
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)

	elapsed := time.Since(start)

	if err != nil {
		t.Skipf("gRPC service not available: %v", err)
		return
	}
	defer conn.Close()

	if elapsed > 1*time.Second {
		t.Errorf("gRPC connection took %v, expected < 1s", elapsed)
	} else {
		t.Logf("✓ gRPC connection latency: %v", elapsed)
	}
}
