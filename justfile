# Start the game: the vector client (omnis-vector), fullscreen, base and test packs, a fresh
# party in the town. Flags pass through: `just run --windowed`, `just run --seed 7`.
run *args:
    cargo run -p omnis-vector -- {{args}}

# The previous client (omnis-app), kept until the vector client reaches parity: character
# creation, saves, town services, inventory and the dev socket live here for now.
run-app *args:
    cargo run -p omnis-app -- {{args}}
