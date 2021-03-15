# osurate - osu! rate generator
Warning: this is still wip; the tool is functional (mostly) but there are still some rough edges to sort out

osurate is a small command line tool written in [Rust](https://www.rust-lang.org/) for generating rates (speed variations) of [osu!](https://osu.ppy.sh) beatmaps. When generating rates, the audio will also be scaled accordingly (and also pitch-shifted).

TODO: gui

## Building
Before building, make sure you have libmp3lame and nightly rustc (at least 1.50.0). To build, just clone [this repo](https://github.com/LunarCoffee/osurate) and compile with `cargo build --release`.

## Usage
```bash
osurate <inputs>... -r <rates>

# This will generate 0.85x and 0.9x rates for Wanderflux [Annihilation].
osurate "Wanderflux [Annihilation].osu" -r 0.85 0.9

# This will generate a 1.1 rate for both specified beatmaps.
osurate "MANIERA [Masterpiece]" "Crystallized [listen]" -r 1.1
```
Specify the paths of the .osu files you want to generate rates for in `inputs`, and put the `rates` you want after.

## Performance
With my Intel i7-6700HQ on Ubuntu, it takes around 2-3 seconds to generate one rate for a 2 minute (~3 MB MP3) map, the bottleneck being the MP3 re-encoding with LAME.
