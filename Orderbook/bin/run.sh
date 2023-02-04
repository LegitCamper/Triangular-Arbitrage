# Runs the Triangular Arbitrage programs

# Kills existing instances
pkill -9 websocket_spawn
pkill -9 websockets.bin
pkill -9 execute_trades.
pkill -9 find_triangular

# Generates catalog
#./create_valid_pairs_catalog.bin

# Downloads symbols        # Finds Arbitrages
./websocket_spawner.bin & ./find_triangular_arbitrage.bin && fg
