#!/bin/bash
set -euo pipefail

# init-validator.sh
# Initializes a DelTran validator node

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DELTRAN_HOME="${DELTRAN_HOME:-$HOME/.deltran}"
CHAIN_ID="${CHAIN_ID:-deltran-mainnet-1}"
MONIKER="${MONIKER:-validator-node}"

echo "🚀 Initializing DelTran Validator"
echo "   Home: $DELTRAN_HOME"
echo "   Chain ID: $CHAIN_ID"
echo "   Moniker: $MONIKER"

# Create directories
mkdir -p "$DELTRAN_HOME"/{config,data}

# Initialize node
deltran-node init \
  --home "$DELTRAN_HOME" \
  --chain-id "$CHAIN_ID" \
  --moniker "$MONIKER"

# Configure CometBFT
CONFIG_FILE="$DELTRAN_HOME/config/config.toml"

# Production-grade settings
cat > "$CONFIG_FILE" <<EOF
# CometBFT Configuration for DelTran

#######################################################################
###                   Main Base Config Options                      ###
#######################################################################

# A custom human readable name for this node
moniker = "$MONIKER"

# Database backend: goleveldb | cleveldb | boltdb | rocksdb | badgerdb
db_backend = "rocksdb"

# Database directory
db_dir = "data"

# Output level for logging, including package level options
log_level = "info"

# Output format: 'plain' (colored text) or 'json'
log_format = "json"

#######################################################################
###                 Advanced Configuration Options                  ###
#######################################################################

#######################################################
###       RPC Server Configuration Options          ###
#######################################################
[rpc]

# TCP or UNIX socket address for the RPC server to listen on
laddr = "tcp://0.0.0.0:26657"

# A list of origins a cross-domain request can be executed from
cors_allowed_origins = []

# Maximum number of simultaneous connections
max_open_connections = 900

# Maximum number of unique clientIDs the WebSocket server will accept
max_subscription_clients = 100

# Maximum number of queries a client can make per request
max_queries_per_request = 20

# How long to wait for a tx to be committed during /broadcast_tx_commit
timeout_broadcast_tx_commit = "10s"

# Maximum size of request body, in bytes
max_body_bytes = 1000000

# Maximum size of request header, in bytes
max_header_bytes = 1048576

# The path to a file containing certificate that is used to create the HTTPS server
tls_cert_file = ""

# The path to a file containing matching private key that is used to create the HTTPS server
tls_key_file = ""

#######################################################
###           P2P Configuration Options             ###
#######################################################
[p2p]

# Address to listen for incoming connections
laddr = "tcp://0.0.0.0:26656"

# Address to advertise to peers for them to dial
external_address = ""

# Comma separated list of seed nodes to connect to
seeds = ""

# Comma separated list of nodes to keep persistent connections to
persistent_peers = ""

# Set true for strict address routability rules
addr_book_strict = true

# Maximum number of inbound peers
max_num_inbound_peers = 40

# Maximum number of outbound peers to connect to, excluding persistent peers
max_num_outbound_peers = 10

# Time to wait before flushing messages out on the connection
flush_throttle_timeout = "100ms"

# Maximum size of a message packet payload, in bytes
max_packet_msg_payload_size = 1024

# Rate at which packets can be sent, in bytes/second
send_rate = 5120000

# Rate at which packets can be received, in bytes/second
recv_rate = 5120000

# Set true to enable the peer-exchange reactor
pex = true

# Seed mode, in which node constantly crawls the network and looks for
# peers. If another node asks it for addresses, it responds and disconnects
seed_mode = false

# Toggle to disable guard against peers connecting from the same ip
allow_duplicate_ip = false

# Peer connection configuration
handshake_timeout = "20s"
dial_timeout = "3s"

#######################################################
###          Mempool Configuration Options          ###
#######################################################
[mempool]

recheck = true
broadcast = true
wal_dir = ""

# Maximum number of transactions in the mempool
size = 5000

# Limit the total size of all txs in the mempool (bytes)
max_txs_bytes = 1073741824

# Size of the cache (used to filter transactions we saw earlier)
cache_size = 10000

# Maximum size of a single transaction
max_tx_bytes = 1048576

#######################################################
###         Consensus Configuration Options         ###
#######################################################
[consensus]

wal_file = "data/cs.wal/wal"

# How long we wait for a proposal block before prevoting nil
timeout_propose = "3s"
# How much timeout_propose increases with each round
timeout_propose_delta = "500ms"
# How long we wait after receiving +2/3 prevotes for "anything"
timeout_prevote = "1s"
# How much the timeout_prevote increases with each round
timeout_prevote_delta = "500ms"
# How long we wait after receiving +2/3 precommits for "anything"
timeout_precommit = "1s"
# How much the timeout_precommit increases with each round
timeout_precommit_delta = "500ms"
# How long we wait after committing a block, before starting on the new height
timeout_commit = "5s"

# Make progress as soon as we have all the precommits (as if TimeoutCommit = 0)
skip_timeout_commit = false

# EmptyBlocks mode and possible interval between empty blocks
create_empty_blocks = true
create_empty_blocks_interval = "0s"

# Reactor sleep duration parameters
peer_gossip_sleep_duration = "100ms"
peer_query_maj23_sleep_duration = "2s"

#######################################################
###   Transaction Indexer Configuration Options     ###
#######################################################
[tx_index]

# What indexer to use for transactions
indexer = "kv"

#######################################################
###       Instrumentation Configuration Options     ###
#######################################################
[instrumentation]

# When true, Prometheus metrics are served under /metrics on
# PrometheusListenAddr
prometheus = true

# Address to listen for Prometheus collector(s) connections
prometheus_listen_addr = ":26660"

# Maximum number of simultaneous connections
max_open_connections = 3

# Instrumentation namespace
namespace = "cometbft"

#######################################################
###           State Sync Configuration              ###
#######################################################
[statesync]

# State sync rapidly bootstraps a new node by discovering, fetching, and restoring a state machine
enable = false

# RPC servers (comma-separated) for light client verification of the synced state machine
rpc_servers = ""

# Trust height for state sync
trust_height = 0

# Trust hash (hex-encoded) for state sync
trust_hash = ""

# Time period during which the trusted snapshot is valid
trust_period = "168h0m0s"

# Time to spend discovering snapshots before initiating a restore
discovery_time = "15s"

# Temporary directory for state sync snapshot chunks
temp_dir = ""

# The timeout duration before re-requesting a chunk, possibly from a different peer
chunk_request_timeout = "10s"

# The number of concurrent chunk fetchers to run
chunk_fetchers = "4"
EOF

echo ""
echo "✅ Validator initialized successfully!"
echo ""
echo "Next steps:"
echo "1. Configure persistent_peers in config.toml"
echo "2. Get genesis.json from network coordinator"
echo "3. Start node: deltran-node start --home $DELTRAN_HOME"
echo ""
