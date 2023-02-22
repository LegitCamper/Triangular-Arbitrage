# Runs the Triangular Arbitrage programs

# Kills existing instances
pkill -9 websocket_spawn
pkill -9 websockets.bin
pkill -9 execute_trades.
pkill -9 find_triangular

# Generates catalog
#./create_valid_pairs_catalog.bin

# Starts websockets
./websocket_spawner.bin &
# Waits for symbols to be downloaded/refreshed
sleep 120
python -c "print('\n\nStarting Arbitrage calculator')"
# Starts Arbitrage calculator
./find_triangular_arbitrage.bin &
# Executes orders for possible Arbitrages
env CURL_CA_BUNDLE="" env REQUESTS_CA_BUNDLE="" ./execute_trades.bin && fg
