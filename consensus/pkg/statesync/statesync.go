package statesync

import (
	"fmt"
	"time"

	cmtcfg "github.com/cometbft/cometbft/config"
)

// Configure sets up state sync for fast bootstrapping
func Configure(config *cmtcfg.Config) error {
	// Enable state sync
	config.StateSync.Enable = true

	// Set RPC servers (should be configured per deployment)
	// These would be replaced with actual trusted RPC endpoints
	config.StateSync.RPCServers = []string{
		"https://rpc1.deltran.network:443",
		"https://rpc2.deltran.network:443",
	}

	// Trust period (2/3 of unbonding period)
	config.StateSync.TrustPeriod = 168 * time.Hour // 7 days

	// Discovery time
	config.StateSync.DiscoveryTime = 15 * time.Second

	// Chunk request timeout
	config.StateSync.ChunkRequestTimeout = 10 * time.Second

	// Number of chunks to fetch concurrently
	config.StateSync.ChunkFetchers = 4

	fmt.Println("✅ State sync configured")
	fmt.Println("   RPC servers:", config.StateSync.RPCServers)
	fmt.Println("   Trust period:", config.StateSync.TrustPeriod)

	return nil
}

// GetTrustedSnapshot returns trusted height and hash for state sync
// In production, this would query trusted validators
func GetTrustedSnapshot() (height int64, hash string, err error) {
	// This is a placeholder - in production this would:
	// 1. Query multiple trusted RPC endpoints
	// 2. Verify consensus among validators
	// 3. Return verified snapshot point

	// For now, return error to indicate manual configuration needed
	return 0, "", fmt.Errorf("trusted snapshot must be configured manually")
}

// SnapshotConfig holds snapshot configuration
type SnapshotConfig struct {
	// Interval between snapshots (blocks)
	Interval uint64

	// Number of snapshots to keep
	KeepRecent uint32

	// Enable async snapshot creation
	Async bool
}

// DefaultSnapshotConfig returns default snapshot configuration
func DefaultSnapshotConfig() *SnapshotConfig {
	return &SnapshotConfig{
		Interval:   1000,  // Every 1000 blocks
		KeepRecent: 2,     // Keep last 2 snapshots
		Async:      true,  // Non-blocking snapshots
	}
}

// ApplySnapshotConfig applies snapshot settings to CometBFT config
func ApplySnapshotConfig(config *cmtcfg.Config, snapConfig *SnapshotConfig) {
	config.StateSync.Enable = true

	// Set snapshot interval
	config.Storage.SnapshotInterval = snapConfig.Interval
	config.Storage.SnapshotKeepRecent = snapConfig.KeepRecent

	fmt.Printf("✅ Snapshot config applied:\n")
	fmt.Printf("   Interval: %d blocks\n", snapConfig.Interval)
	fmt.Printf("   Keep recent: %d snapshots\n", snapConfig.KeepRecent)
}
