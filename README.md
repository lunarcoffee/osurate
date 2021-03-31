# osurate - osu! rate generator

osurate is a small command line tool (optional GUI available) written in [Rust](https://www.rust-lang.org/) for
generating rates (speed variations) of [osu!](https://osu.ppy.sh) beatmaps. When generating rates, the audio will also
be scaled accordingly (and pitch-shifted).

## Building

If you're on Windows, you can download the latest release [here](https://github.com/LunarCoffee/osurate/releases). This
will include a binary executable, a launch script that enters the GUI, as well as usage instructions. That's it!

Otherwise, before building, make sure you have libmp3lame and nightly rustc (at least 1.50.0). If you want to build with GUI
support on Linux, also have GTK+ 3 installed. To build, just clone [this repo](https://github.com/LunarCoffee/osurate)
and compile with `cargo build --release`, and tack on `--features gui` if you want the GUI.

## Usage

```shell
osurate <inputs>... -r <rates>

# This will generate 0.85x and 0.9x rates for the specified map.
osurate "Wanderflux [Annihilation].osu" -r 0.85 0.9

# This will generate 1.1x and 1.2x rates for both specified maps.
osurate "MANIERA [Collab Another]" "Crystallized [listen]" -r 1.1 1.2

# This opens the GUI.
osurate -g
```

When using the CLI, specify the paths of the .osu files you want to generate rates for in `inputs`, and put the `rates`
you want after. If you specify multiple files, all of the rates you specify will be generated for each file.

## Performance

With an Intel i7-6700HQ on Ubuntu, it takes around 2-3 seconds to generate one rate for a 2 minute (~3 MB MP3) map, the
bottleneck being the MP3 encoding with LAME.
