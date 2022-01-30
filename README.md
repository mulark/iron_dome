# Iron Dome

Factorio artillery clicker

## Deps
* Rust (Obviously)
* Need xdotool installed

## Usage
``` cargo run --release ```

Will click anything that looks like a biter nest (spawner/worm) on the map. Minimal false positives, but possible. Uses improved targetting algorithm that results in fewer artillery shells used than Vanilla targetting/ most other autoclickers, and can likely find better solutions than humans on big nests. (at least, drastically faster).

If zoomed in enough on the map view and playing with the show-active-state debug option turned on, it will classify the biter nests based on the debug graphics (Magenta/Blue circles). Otherwise, it attempts to classify the nests based on red pixels that roughly look like worms/nests based on the w x h ratio of the seen red pixels.
