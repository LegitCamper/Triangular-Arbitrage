# Runs the Triangular Arbitrage programs

# Generates catalog
#./create_valid_pairs_catalog.bin

# Downloads symbols        # Finds Arbitrages
./websocket_spawner.bin & ./find_triangular_arbitrage.bin && fg
