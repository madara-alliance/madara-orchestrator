# Orchestrator
ORCHESTRATOR_PATH := $(shell pwd)

# Bootstrapper
OPERATOR_ADDRESS := 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
BOOTSTRAP_OUTPUT_PATH := $(shell pwd)/build/bootstrap.json
BOOTSTRAPPER_COMMIT := b0b647500c2ae3e3b0d99e345fa652989bca4726
BOOTSTRAPPER_PATH := $(shell pwd)/madara-bootstrapper
VERIFIER_ADDRESS := 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512

# SNOS
CAIRO_LANG_COMMIT := a86e92bfde9c171c0856d7b46580c66e004922f3

# Madara
MADARA_PATH := $(shell pwd)/madara
MADARA_COMMIT := ad0ec8cdfcde03a8e2feef76d64a7fba7a9fb792
MADARA_DATA_PATH := $(shell pwd)/build/madara

# Pathfinder
PATHFINDER_COMMIT := 138140e5fd967ede92806fd62de47c2e6b65712a
PATHFINDER_DATA_PATH := $(shell pwd)/build/pathfinder
PATHFINDER_PATH := $(shell pwd)/pathfinder

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

define update_core_contract_address
	echo "Updating core contract address in YAML..."
	if [ -f "$(ENV_FILE)" ]; then \
		. $(ENV_FILE); \
		if [ -n "$$CORE_CONTRACT_ADDRESS" ]; then \
			export CORE_CONTRACT_ADDRESS=$$CORE_CONTRACT_ADDRESS && \
			yq e '.eth_core_contract_address = env(CORE_CONTRACT_ADDRESS)' -i $(MADARA_PATH)/configs/presets/devnet.yaml; \
			echo "Core contract address updated in YAML."; \
		else \
			echo "Error: CORE_CONTRACT_ADDRESS not found in $(ENV_FILE)"; \
			exit 1; \
		fi; \
	else \
		echo "Error: $(ENV_FILE) not found"; \
		exit 1; \
	fi
endef



madara-bootstrap-mode:
	$(call update_core_contract_address) && \
	cd $(MADARA_PATH) && \
	git checkout $(MADARA_COMMIT) && \
	rm -rf $(MADARA_DATA_PATH) && \
	cargo run --release -- --name madara --base-path $(MADARA_DATA_PATH) --rpc-port 9944 --rpc-cors "*" --rpc-external --sequencer --chain-config-path configs/presets/devnet.yaml --feeder-gateway-enable --gateway-enable --gateway-external --gas-price 0 --blob-gas-price 0 --rpc-methods unsafe --no-l1-sync

core-contract:
	cd $(BOOTSTRAPPER_PATH) && \
	rm -f $(BOOTSTRAP_OUTPUT_PATH) && \
	git checkout $(BOOTSTRAPPER_COMMIT) && \
	RUST_LOG=debug cargo run --release -- --mode core --operator-address $(OPERATOR_ADDRESS) --output-file $(BOOTSTRAP_OUTPUT_PATH) --verifier-address $(VERIFIER_ADDRESS) && \
	echo "CORE_CONTRACT_ADDRESS=$$(jq -r .starknet_contract_address $(BOOTSTRAP_OUTPUT_PATH))" >> $(ENV_FILE) && \
	echo "CORE_CONTRACT_IMPLEMENTATION_ADDRESS=$$(jq -r .starknet_contract_implementation_address $(BOOTSTRAP_OUTPUT_PATH))" >> $(ENV_FILE)

madara:
	$(call update_core_contract_address) && \
	cd $(MADARA_PATH) && \
	git checkout $(MADARA_COMMIT) && \
	cargo run --release -- --name madara --base-path $(MADARA_DATA_PATH) --rpc-port 9944 --rpc-cors "*" --rpc-external --sequencer --chain-config-path configs/presets/devnet.yaml --feeder-gateway-enable --gateway-enable --gateway-external --gas-price 2 --blob-gas-price 2 --rpc-methods unsafe --l1-endpoint http://0.0.0.0:8545

eth-bridge:
	cd $(BOOTSTRAPPER_PATH) && \
	git checkout $(BOOTSTRAPPER_COMMIT) && \
	. $(ENV_FILE) && \
	echo "TODO: set core contract address" && \
	RUST_LOG=debug cargo run --release -- --mode eth-bridge --core-contract-address $$CORE_CONTRACT_ADDRESS --core-contract-implementation-address $$CORE_CONTRACT_IMPLEMENTATION_ADDRESS  --output-file $(BOOTSTRAP_OUTPUT_PATH) && \
	echo "L1_BRIDGE_ADDRESS=$$(jq -r .eth_bridge_setup_outputs.l1_bridge_address $(BOOTSTRAP_OUTPUT_PATH))" >> $(ENV_FILE)
	echo "L2_ETH_TOKEN_ADDRESS=$$(jq -r .eth_bridge_setup_outputs.l2_eth_proxy_address $(BOOTSTRAP_OUTPUT_PATH))" >> $(ENV_FILE) && \
	echo "L2_ETH_BRIDGE_ADDRESS=$$(jq -r .eth_bridge_setup_outputs.l2_eth_bridge_proxy_address $(BOOTSTRAP_OUTPUT_PATH))" >> $(ENV_FILE)

udc:
	cd $(BOOTSTRAPPER_PATH) && \
	git checkout $(BOOTSTRAPPER_COMMIT) && \
	RUST_LOG=debug cargo run --release -- --mode udc

pathfinder:
	cd $(PATHFINDER_PATH) && \
	git checkout $(PATHFINDER_COMMIT) && \
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
	cd $(ORCHESTRATOR_PATH) && \
	cargo run --release --bin orchestrator

snos:
	python3 -m venv orchestrator_venv && \
	source orchestrator_venv/bin/activate && \
	pip install cairo-lang==0.13.2 "sympy<1.13.0" && \
	mkdir -p build && \
	git submodule update --init --recursive && \
	cd cairo-lang && \
	git checkout $(CAIRO_LANG_COMMIT) && \
	cd .. && \
	cairo-compile cairo-lang/src/starkware/starknet/core/os/os.cairo --output build/os_latest.json --cairo_path cairo-lang/src

cleanup:
	@echo "Cleaning up processes..."
	@-kill $$(cat .madara_pid 2>/dev/null) 2>/dev/null || true
	@-kill $$(cat .pathfinder_pid 2>/dev/null) 2>/dev/null || true
	@-pkill -f "cargo run --release -- --name madara" 2>/dev/null || true
	@-pkill -f "cargo run --release --bin pathfinder" 2>/dev/null || true
	@rm -f .madara_pid .pathfinder_pid
	@rm -f $(ENV_FILE)
	@echo "Cleanup completed."