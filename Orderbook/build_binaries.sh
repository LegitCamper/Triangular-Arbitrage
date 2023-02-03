# Builds binaries

# Have Clang installed via pacman
# Have nuitka installed via pip

# Change cache dir
export NUITKA_CACHE_DIR=~/Documents/nuitka

# Main Files
nuitka3 --quiet --output-dir=bin --clang websockets.py
nuitka3 --quiet --output-dir=bin --clang websocket_spawner.py
nuitka3 --quiet --output-dir=bin --clang find_triangular_arbitrage.py
nuitka3 --quiet --output-dir=bin --clang execute_trades.py 

# Helper files
#nuitka3 --quiet --remove-output --output-dir=bin --clang create_valid_pairs_catalog.py
