package genesis

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"time"

	"github.com/cometbft/cometbft/types"
)

// Generator creates genesis files for DelTran network
type Generator struct {
	chainID       string
	genesisTime   time.Time
	validators    []ValidatorInfo
	initialHeight int64
}

// ValidatorInfo holds validator configuration
type ValidatorInfo struct {
	Name      string
	PubKey    string
	Power     int64
	Address   string
}

// NewGenerator creates a new genesis generator
func NewGenerator() *Generator {
	return &Generator{
		chainID:       "deltran-mainnet-1",
		genesisTime:   time.Now(),
		validators:    make([]ValidatorInfo, 0),
		initialHeight: 1,
	}
}

// WithChainID sets the chain ID
func (g *Generator) WithChainID(id string) *Generator {
	g.chainID = id
	return g
}

// WithGenesisTime sets genesis time
func (g *Generator) WithGenesisTime(t time.Time) *Generator {
	g.genesisTime = t
	return g
}

// AddValidator adds a validator to genesis
func (g *Generator) AddValidator(info ValidatorInfo) *Generator {
	g.validators = append(g.validators, info)
	return g
}

// Generate creates and writes genesis file
func (g *Generator) Generate(homeDir string) error {
	// Create genesis doc
	genDoc := &types.GenesisDoc{
		ChainID:       g.chainID,
		GenesisTime:   g.genesisTime,
		InitialHeight: g.initialHeight,
		ConsensusParams: &types.ConsensusParams{
			Block: types.BlockParams{
				MaxBytes:   22020096, // 21MB
				MaxGas:     -1,       // No gas limit
				TimeIotaMs: 1000,     // 1 second
			},
			Evidence: types.EvidenceParams{
				MaxAgeNumBlocks: 100000,
				MaxAgeDuration:  172800000000000, // 48 hours
				MaxBytes:        1048576,         // 1MB
			},
			Validator: types.ValidatorParams{
				PubKeyTypes: []string{"ed25519"},
			},
			Version: types.VersionParams{
				AppVersion: 1,
			},
		},
		AppState: g.buildAppState(),
	}

	// Add validators
	for _, val := range g.validators {
		pubKey, err := types.NewPubKeyFromBase64(val.PubKey)
		if err != nil {
			return fmt.Errorf("invalid validator pubkey: %w", err)
		}

		genDoc.Validators = append(genDoc.Validators, types.GenesisValidator{
			Address: pubKey.Address(),
			PubKey:  pubKey,
			Power:   val.Power,
			Name:    val.Name,
		})
	}

	// Validate
	if err := genDoc.ValidateAndComplete(); err != nil {
		return fmt.Errorf("invalid genesis: %w", err)
	}

	// Write to file
	genFile := filepath.Join(homeDir, "config", "genesis.json")
	if err := genDoc.SaveAs(genFile); err != nil {
		return fmt.Errorf("failed to save genesis: %w", err)
	}

	fmt.Printf("✅ Genesis file written to: %s\n", genFile)
	return nil
}

// buildAppState creates DelTran-specific application state
func (g *Generator) buildAppState() json.RawMessage {
	appState := map[string]interface{}{
		"chain_id": g.chainID,
		"network":  "mainnet",
		"protocol_version": 1,
		"features": map[string]bool{
			"netting_enabled":     true,
			"partial_settlement":  true,
			"fx_orchestration":    true,
			"compliance_hooks":    true,
			"regulatory_reporting": true,
		},
		"limits": map[string]interface{}{
			"max_payment_amount":       "100000000.00", // 100M USD
			"min_netting_volume":       "100000.00",    // 100K USD
			"min_netting_efficiency":   0.15,           // 15%
			"settlement_window_hours":  6,
			"max_corridor_participants": 50,
		},
		"security": map[string]interface{}{
			"hsm_enabled":           true,
			"checkpoint_interval":   100,
			"bft_quorum_threshold":  "5/7",
			"signature_algorithm":   "ed25519",
		},
	}

	data, _ := json.MarshalIndent(appState, "", "  ")
	return json.RawMessage(data)
}

// LoadValidatorsFromDir loads validator info from directory
func LoadValidatorsFromDir(dir string) ([]ValidatorInfo, error) {
	validators := make([]ValidatorInfo, 0)

	// Read all validator-*.json files
	files, err := filepath.Glob(filepath.Join(dir, "validator-*.json"))
	if err != nil {
		return nil, err
	}

	for _, file := range files {
		data, err := os.ReadFile(file)
		if err != nil {
			return nil, fmt.Errorf("failed to read %s: %w", file, err)
		}

		var val ValidatorInfo
		if err := json.Unmarshal(data, &val); err != nil {
			return nil, fmt.Errorf("failed to parse %s: %w", file, err)
		}

		validators = append(validators, val)
	}

	return validators, nil
}
