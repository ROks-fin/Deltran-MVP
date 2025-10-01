package main

import (
	"fmt"
	"os"
	"os/signal"
	"path/filepath"
	"syscall"

	"github.com/cometbft/cometbft/abci/server"
	cmtcfg "github.com/cometbft/cometbft/config"
	cmtlog "github.com/cometbft/cometbft/libs/log"
	"github.com/cometbft/cometbft/node"
	"github.com/cometbft/cometbft/p2p"
	"github.com/cometbft/cometbft/privval"
	"github.com/cometbft/cometbft/proxy"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
	"go.uber.org/zap"

	"github.com/deltran/consensus/pkg/genesis"
	"github.com/deltran/consensus/pkg/keymanager"
	"github.com/deltran/consensus/pkg/statesync"
)

var (
	rootCmd = &cobra.Command{
		Use:   "deltran-node",
		Short: "DelTran Consensus Node",
		Long:  "Byzantine Fault Tolerant consensus node for DelTran settlement rail",
	}

	startCmd = &cobra.Command{
		Use:   "start",
		Short: "Start consensus node",
		RunE:  startNode,
	}

	initCmd = &cobra.Command{
		Use:   "init",
		Short: "Initialize node configuration and keys",
		RunE:  initNode,
	}

	genGenesisCmd = &cobra.Command{
		Use:   "gen-genesis",
		Short: "Generate genesis file",
		RunE:  generateGenesis,
	}
)

func init() {
	rootCmd.AddCommand(startCmd, initCmd, genGenesisCmd)

	// Flags
	rootCmd.PersistentFlags().String("home", "", "node home directory")
	rootCmd.PersistentFlags().String("config", "", "config file path")

	startCmd.Flags().String("moniker", "", "node moniker")
	startCmd.Flags().String("proxy-app", "tcp://127.0.0.1:26658", "ABCI application address")
	startCmd.Flags().Bool("state-sync", false, "enable state sync")

	initCmd.Flags().String("chain-id", "deltran-mainnet-1", "chain ID")
	initCmd.Flags().String("moniker", "", "node moniker (required)")
	initCmd.MarkFlagRequired("moniker")

	viper.BindPFlags(rootCmd.PersistentFlags())
	viper.BindPFlags(startCmd.Flags())
	viper.BindPFlags(initCmd.Flags())
}

func main() {
	if err := rootCmd.Execute(); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}

func startNode(cmd *cobra.Command, args []string) error {
	logger, err := zap.NewProduction()
	if err != nil {
		return fmt.Errorf("failed to create logger: %w", err)
	}
	defer logger.Sync()

	homeDir := getHomeDir()
	logger.Info("Starting DelTran consensus node", zap.String("home", homeDir))

	// Load CometBFT config
	cmtConfig := cmtcfg.DefaultConfig()
	cmtConfig.SetRoot(homeDir)

	if err := viper.Unmarshal(cmtConfig); err != nil {
		return fmt.Errorf("failed to unmarshal config: %w", err)
	}

	// Load or create node key
	nodeKey, err := p2p.LoadOrGenNodeKey(cmtConfig.NodeKeyFile())
	if err != nil {
		return fmt.Errorf("failed to load node key: %w", err)
	}

	// Load private validator
	pv := privval.LoadOrGenFilePV(
		cmtConfig.PrivValidatorKeyFile(),
		cmtConfig.PrivValidatorStateFile(),
	)

	// Create ABCI client
	proxyApp := viper.GetString("proxy-app")
	clientCreator := proxy.NewRemoteClientCreator(proxyApp, "socket", true)

	// Create logger adapter
	cmtLogger := cmtlog.NewTMLogger(os.Stdout)

	// State sync setup
	if viper.GetBool("state-sync") {
		logger.Info("State sync enabled")
		if err := statesync.Configure(cmtConfig); err != nil {
			return fmt.Errorf("failed to configure state sync: %w", err)
		}
	}

	// Create node
	n, err := node.NewNode(
		cmtConfig,
		pv,
		nodeKey,
		clientCreator,
		node.DefaultGenesisDocProviderFunc(cmtConfig),
		cmtcfg.DefaultDBProvider,
		node.DefaultMetricsProvider(cmtConfig.Instrumentation),
		cmtLogger,
	)
	if err != nil {
		return fmt.Errorf("failed to create node: %w", err)
	}

	// Start node
	if err := n.Start(); err != nil {
		return fmt.Errorf("failed to start node: %w", err)
	}

	logger.Info("Node started successfully",
		zap.String("nodeID", string(nodeKey.ID())),
		zap.String("p2p", cmtConfig.P2P.ListenAddress),
		zap.String("rpc", cmtConfig.RPC.ListenAddress),
	)

	// Wait for shutdown signal
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, os.Interrupt, syscall.SIGTERM)
	<-sigCh

	logger.Info("Shutting down node...")
	if err := n.Stop(); err != nil {
		return fmt.Errorf("failed to stop node: %w", err)
	}

	n.Wait()
	logger.Info("Node stopped")
	return nil
}

func initNode(cmd *cobra.Command, args []string) error {
	homeDir := getHomeDir()
	chainID := viper.GetString("chain-id")
	moniker := viper.GetString("moniker")

	logger, _ := zap.NewProduction()
	defer logger.Sync()

	logger.Info("Initializing node",
		zap.String("home", homeDir),
		zap.String("chain-id", chainID),
		zap.String("moniker", moniker),
	)

	// Create directories
	dirs := []string{
		filepath.Join(homeDir, "config"),
		filepath.Join(homeDir, "data"),
	}
	for _, dir := range dirs {
		if err := os.MkdirAll(dir, 0755); err != nil {
			return fmt.Errorf("failed to create directory %s: %w", dir, err)
		}
	}

	// Initialize CometBFT config
	cmtConfig := cmtcfg.DefaultConfig()
	cmtConfig.SetRoot(homeDir)
	cmtConfig.Moniker = moniker

	// Generate node key
	nodeKey, err := p2p.LoadOrGenNodeKey(cmtConfig.NodeKeyFile())
	if err != nil {
		return fmt.Errorf("failed to generate node key: %w", err)
	}

	// Generate validator key
	km := keymanager.New(homeDir)
	if err := km.GenerateValidatorKey(); err != nil {
		return fmt.Errorf("failed to generate validator key: %w", err)
	}

	// Write config
	cmtcfg.WriteConfigFile(filepath.Join(homeDir, "config", "config.toml"), cmtConfig)

	logger.Info("Node initialized successfully",
		zap.String("node-id", string(nodeKey.ID())),
		zap.String("validator-address", km.GetValidatorAddress()),
	)

	fmt.Printf("\n✅ Node initialized successfully\n")
	fmt.Printf("   Home: %s\n", homeDir)
	fmt.Printf("   Node ID: %s\n", nodeKey.ID())
	fmt.Printf("   Validator Address: %s\n", km.GetValidatorAddress())
	fmt.Printf("\nNext steps:\n")
	fmt.Printf("1. Add validator info to genesis.json\n")
	fmt.Printf("2. Configure persistent peers\n")
	fmt.Printf("3. Start node: deltran-node start\n\n")

	return nil
}

func generateGenesis(cmd *cobra.Command, args []string) error {
	homeDir := getHomeDir()

	logger, _ := zap.NewProduction()
	defer logger.Sync()

	logger.Info("Generating genesis file", zap.String("home", homeDir))

	gen := genesis.NewGenerator()
	if err := gen.Generate(homeDir); err != nil {
		return fmt.Errorf("failed to generate genesis: %w", err)
	}

	logger.Info("Genesis file generated successfully")
	return nil
}

func getHomeDir() string {
	home := viper.GetString("home")
	if home == "" {
		home = os.Getenv("DELTRAN_HOME")
	}
	if home == "" {
		home = filepath.Join(os.Getenv("HOME"), ".deltran")
	}
	return home
}
