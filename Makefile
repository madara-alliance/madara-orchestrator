# Configurable paths
MADARA_PATH := /Users/apoorvsadana/Documents/GitHub/madara
BOOTSTRAPPER_PATH := /Users/apoorvsadana/Documents/GitHub/kanot-bridge-deployment
PATHFINDER_PATH := /Users/apoorvsadana/Documents/GitHub/pathfinder
ORCHESTRATOR_PATH := /Users/apoorvsadana/Documents/GitHub/madara-orchestrator
BOOTSTRAP_JSON_PATH := $(BOOTSTRAPPER_PATH)/bootstrap.json
PATHFINDER_DATA_PATH := $(PATHFINDER_PATH)/test_db
MADARA_BASE_PATH := ../madara_pathfinder_test_15
OPERATOR_ADDRESS := 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266

# Environment file
ENV_FILE := $(shell pwd)/.makefile-env

.PHONY: all anvil madara core-contract update-madara eth-bridge pathfinder orchestrator cleanup

setup: 
	@echo "Starting setup process..."
	@make cleanup
	@trap 'make cleanup' EXIT INT TERM
	@make anvil & echo $$! > .anvil_pid
	@echo "Anvil started in background (PID: $$(cat .anvil_pid))."
	@echo "Starting setup process..."
	@make madara-bootstrap-mode & echo $$! > .madara_pid
	@echo "Madara started in background (PID: $$(cat .madara_pid))."
	@make core-contract
	@echo "Core contract setup completed."
	@make udc
	@echo "UDC deployed on chain."
	@kill $$(cat .madara_pid) 2>/dev/null || true
	@echo "Previous Madara instance terminated."
	@echo "Please update core contract address"
	@read -p "Press Enter to continue..."
	@echo "Starting ETH Bridge setup..."
	@make eth-bridge
	@echo "ETH Bridge setup completed."
	@make madara & echo $$! > .madara_pid
	@echo "Updated Madara started in background (PID: $$(cat .madara_pid))."
	@make pathfinder & echo $$! > .pathfinder_pid
	@echo "Pathfinder started in background (PID: $$(cat .pathfinder_pid))."
	@echo "Setting up Madara orchestrator"
	@make orchestrator-setup
	@echo "Madara orchestrator setup completed."
	@echo "Adding indexes to MongoDB"
	@make mongo-migrations
	@echo "Indexes added to MongoDB"
	@echo "Starting orchestrator"
	@make orchestrator
	@echo "Setup completed."
	@make cleanup

anvil:
	anvil --block-time 6

madara-bootstrap-mode:
	cd $(MADARA_PATH) && \
	rm -rf $(MADARA_BASE_PATH) && \
	cargo run --release -- --name madara --base-path $(MADARA_BASE_PATH) --rpc-port 9944 --rpc-cors "*" --rpc-external --sequencer --chain-config-path configs/presets/devnet.yaml --feeder-gateway-enable --gateway-enable --gateway-external --gas-price 0 --blob-gas-price 0 --rpc-methods unsafe --no-l1-sync

core-contract:
	cd $(BOOTSTRAPPER_PATH) && \
	rm -f $(BOOTSTRAP_JSON_PATH) && \
	RUST_LOG=debug cargo run --release -- --mode core --operator-address $(OPERATOR_ADDRESS) --output-file $(BOOTSTRAP_JSON_PATH) && \
	echo "CORE_CONTRACT_ADDRESS=$$(jq -r .starknet_contract_address $(BOOTSTRAP_JSON_PATH))" >> $(ENV_FILE) && \
	echo "CORE_CONTRACT_IMPLEMENTATION_ADDRESS=$$(jq -r .starknet_contract_implementation_address $(BOOTSTRAP_JSON_PATH))" >> $(ENV_FILE)

madara:
	cd $(MADARA_PATH) && \
	cargo run --release -- --name madara --base-path $(MADARA_BASE_PATH) --rpc-port 9944 --rpc-cors "*" --rpc-external --sequencer --chain-config-path configs/presets/devnet.yaml --feeder-gateway-enable --gateway-enable --gateway-external --gas-price 2 --blob-gas-price 2 --rpc-methods unsafe --l1-endpoint http://0.0.0.0:8545

eth-bridge:
	cd $(BOOTSTRAPPER_PATH) && \
	. $(ENV_FILE) && \
	RUST_LOG=debug cargo run --release -- --mode eth-bridge --core-contract-address $$CORE_CONTRACT_ADDRESS --core-contract-implementation-address $$CORE_CONTRACT_IMPLEMENTATION_ADDRESS  --output-file $(BOOTSTRAP_JSON_PATH) && \
	echo "L1_BRIDGE_ADDRESS=$$(jq -r .eth_bridge_setup_outputs.l1_bridge_address $(BOOTSTRAP_JSON_PATH))" >> $(ENV_FILE)
	echo "L2_ETH_TOKEN_ADDRESS=$$(jq -r .eth_bridge_setup_outputs.l2_eth_proxy_address $(BOOTSTRAP_JSON_PATH))" >> $(ENV_FILE) && \
	echo "L2_ETH_BRIDGE_ADDRESS=$$(jq -r .eth_bridge_setup_outputs.l2_eth_bridge_proxy_address $(BOOTSTRAP_JSON_PATH))" >> $(ENV_FILE)

udc:
	cd $(BOOTSTRAPPER_PATH) && \
	RUST_LOG=debug cargo run --release -- --mode udc

pathfinder:
	cd $(PATHFINDER_PATH) && \
	rm -rf $(PATHFINDER_DATA_PATH) && \
	cargo run --release --bin pathfinder -- --ethereum.url wss://eth-sepolia.g.alchemy.com/v2/WIUR5JUZXieEBkze6Xs3IOXWhsS840TX --chain-id MADARA_DEVNET --feeder-gateway-url http://localhost:8080/feeder_gateway --gateway-url  http://localhost:8080/gateway --network custom --storage.state-tries archive --data-directory $(PATHFINDER_DATA_PATH)

mongo-migrations:
	cd $(ORCHESTRATOR_PATH) && \
	npm i && \
	npm run migrate up

orchestrator-setup:
	. $(ENV_FILE) && \
	cd $(ORCHESTRATOR_PATH) && \
	npm i && \
	node scripts/init_state.js $$L1_BRIDGE_ADDRESS $$CORE_CONTRACT_ADDRESS $$L2_ETH_TOKEN_ADDRESS $$L2_ETH_BRIDGE_ADDRESS

orchestrator: 
	. $(ENV_FILE) 
	@echo "Please update core contract"
	@read -p "Press Enter to continue..."
	cd $(ORCHESTRATOR_PATH) && \
	cargo run --release --bin orchestrator

cleanup:
	@echo "Cleaning up processes..."
	@-kill $$(cat .madara_pid 2>/dev/null) 2>/dev/null || true
	@-kill $$(cat .pathfinder_pid 2>/dev/null) 2>/dev/null || true
	@-pkill -f "cargo run --release -- --name madara" 2>/dev/null || true
	@-pkill -f "cargo run --release --bin pathfinder" 2>/dev/null || true
	@rm -f .madara_pid .pathfinder_pid
	@rm -f $(ENV_FILE)
	@echo "Cleanup completed."