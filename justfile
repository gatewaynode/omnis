# Start the game: fullscreen on the current monitor, base pack, quick save in .omnis/.
# Flags pass through: `just run --window medium`, `just run --seed 7`.
run *args:
    cargo run -p omnis-app -- {{args}}
