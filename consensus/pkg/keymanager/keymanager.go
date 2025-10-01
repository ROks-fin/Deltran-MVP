package keymanager

import (
	"crypto/ed25519"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"time"

	"github.com/cometbft/cometbft/crypto"
	cmted25519 "github.com/cometbft/cometbft/crypto/ed25519"
	"github.com/cometbft/cometbft/privval"
)

// KeyManager handles validator key management
type KeyManager struct {
	homeDir string
}

// New creates a new KeyManager
func New(homeDir string) *KeyManager {
	return &KeyManager{homeDir: homeDir}
}

// GenerateValidatorKey generates a new validator keypair
func (km *KeyManager) GenerateValidatorKey() error {
	privKeyFile := filepath.Join(km.homeDir, "config", "priv_validator_key.json")
	stateFile := filepath.Join(km.homeDir, "data", "priv_validator_state.json")

	// Check if key already exists
	if _, err := os.Stat(privKeyFile); err == nil {
		return fmt.Errorf("validator key already exists at %s", privKeyFile)
	}

	// Generate Ed25519 keypair
	pubKey, privKey, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		return fmt.Errorf("failed to generate key: %w", err)
	}

	// Convert to CometBFT key format
	cmtPrivKey := cmted25519.PrivKey(privKey)
	cmtPubKey := cmtPrivKey.PubKey()

	// Create priv_validator_key.json
	privKeyJSON := privval.FilePVKey{
		Address: cmtPubKey.Address(),
		PubKey:  cmtPubKey,
		PrivKey: cmtPrivKey,
	}

	privKeyData, err := json.MarshalIndent(privKeyJSON, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal key: %w", err)
	}

	if err := os.WriteFile(privKeyFile, privKeyData, 0600); err != nil {
		return fmt.Errorf("failed to write key file: %w", err)
	}

	// Create priv_validator_state.json
	stateJSON := privval.FilePVLastSignState{
		Height:    0,
		Round:     0,
		Step:      0,
		Signature: nil,
		SignBytes: nil,
	}

	stateData, err := json.MarshalIndent(stateJSON, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal state: %w", err)
	}

	if err := os.WriteFile(stateFile, stateData, 0600); err != nil {
		return fmt.Errorf("failed to write state file: %w", err)
	}

	fmt.Printf("✅ Validator key generated:\n")
	fmt.Printf("   Address: %s\n", cmtPubKey.Address())
	fmt.Printf("   PubKey: %s\n", hex.EncodeToString(pubKey))
	fmt.Printf("   Key file: %s\n", privKeyFile)

	return nil
}

// GetValidatorAddress returns the validator address
func (km *KeyManager) GetValidatorAddress() string {
	privKeyFile := filepath.Join(km.homeDir, "config", "priv_validator_key.json")

	data, err := os.ReadFile(privKeyFile)
	if err != nil {
		return "N/A"
	}

	var key privval.FilePVKey
	if err := json.Unmarshal(data, &key); err != nil {
		return "N/A"
	}

	return key.Address.String()
}

// RotateKey implements key rotation without downtime
func (km *KeyManager) RotateKey() error {
	// Backup old key
	privKeyFile := filepath.Join(km.homeDir, "config", "priv_validator_key.json")
	backupFile := fmt.Sprintf("%s.backup.%d", privKeyFile, time.Now().Unix())

	data, err := os.ReadFile(privKeyFile)
	if err != nil {
		return fmt.Errorf("failed to read key: %w", err)
	}

	if err := os.WriteFile(backupFile, data, 0600); err != nil {
		return fmt.Errorf("failed to backup key: %w", err)
	}

	// Generate new key
	if err := os.Remove(privKeyFile); err != nil {
		return fmt.Errorf("failed to remove old key: %w", err)
	}

	if err := km.GenerateValidatorKey(); err != nil {
		// Restore backup on failure
		os.Rename(backupFile, privKeyFile)
		return fmt.Errorf("failed to generate new key: %w", err)
	}

	fmt.Printf("✅ Key rotated successfully. Backup: %s\n", backupFile)
	return nil
}

// ExportPublicKey exports public key for genesis
func (km *KeyManager) ExportPublicKey() (crypto.PubKey, error) {
	privKeyFile := filepath.Join(km.homeDir, "config", "priv_validator_key.json")

	data, err := os.ReadFile(privKeyFile)
	if err != nil {
		return nil, fmt.Errorf("failed to read key: %w", err)
	}

	var key privval.FilePVKey
	if err := json.Unmarshal(data, &key); err != nil {
		return nil, fmt.Errorf("failed to parse key: %w", err)
	}

	return key.PubKey, nil
}

// ValidatorInfo exports validator info for genesis
type ValidatorInfo struct {
	Name      string         `json:"name"`
	Address   string         `json:"address"`
	PubKey    crypto.PubKey  `json:"pub_key"`
	Power     int64          `json:"power"`
}

// ExportValidatorInfo exports complete validator info
func (km *KeyManager) ExportValidatorInfo(name string, power int64) (*ValidatorInfo, error) {
	pubKey, err := km.ExportPublicKey()
	if err != nil {
		return nil, err
	}

	return &ValidatorInfo{
		Name:    name,
		Address: pubKey.Address().String(),
		PubKey:  pubKey,
		Power:   power,
	}, nil
}
