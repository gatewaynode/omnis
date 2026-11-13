# Start the game: the client (package omnis-vector, binary omnis), fullscreen, the base pack and the owner's world (packs/the_lower_kingdoms), at the
# title. Flags pass through: `just run --windowed`, `just run --seed 7` (straight into a game).
run *args:
    cargo run -p omnis-vector -- {{args}}

# Start a game on one of the test fixtures' maps, in a window: what the checks and captures of the
# 3D view are made on. `just qa` is the yard (objects, piles, a being); `just qa heights:map:hills`
# (relief), `just qa levels:map:tower` (levels and joins), `just qa test:map:town` (doors, portals).
# More flags pass through: `just qa objects:map:yard --settings .omnis/test-saves/still-low.ron`.
qa map="objects:map:yard" *args:
    cargo run -p omnis-vector -- --windowed --pack packs/base --pack crates/omnis-sim/tests/packs/test --pack crates/omnis-sim/tests/packs/heights --pack crates/omnis-sim/tests/packs/levels --pack crates/omnis-sim/tests/packs/objects --seed 1 --entry {{map}} {{args}}
