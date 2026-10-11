# Start the game: the client (package omnis-vector, binary omnis), fullscreen, base and test packs, at the
# title. Flags pass through: `just run --windowed`, `just run --seed 7` (straight into a game).
run *args:
    cargo run -p omnis-vector -- {{args}}
