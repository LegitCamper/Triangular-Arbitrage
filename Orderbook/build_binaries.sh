# Builds binaries

# Have Clang installed via pacman
# Have nuitka installed via pip

# Main Files
nuitka3 --quiet --remove-output --output-dir=bin websockets.py
nuitka3 --quiet --remove-output --output-dir=bin websocket_spawner.py
nuitka3 --quiet --remove-output --output-dir=bin find_triangular_arbitrage.py

# Helper files
nuitka3 --quiet --remove-output --output-dir=bin create_valid_pairs_catalog.py
