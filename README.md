# Filephile

A customisable command-line file manager written in rust

## Installation

### Windows

1. Download the executable from the latest release on [github](https://github.com/micnekr/filephile/releases)

### Linux

Use a package manager or [install from source ](#install-from-source)

<a id="install-from-source"></a>

### Building from source

1. Install [rust](https://www.rust-lang.org/tools/install) preferably using `rustup`, as well as git
2. Download a copy of the code: `git clone https://github.com/micnekr/filephile.git`
3. Go to the code folder: `cd filephile`
4. Build the project: `cargo build --release`
5. Copy `./target/release/fphile` to somewhere that is in your `PATH`. On a Unix-like OS, a good candidate is `/usr/local/bin` or `/usr/bin`. Note that `/usr/local/bin` may not be included in `PATH` by default and you might need to add it. On Windows, you can place it under the `C:\Program Files` folder.

## Making the config files

If you installed manually (i.e. not from a package manager), you will need to use a config file. I recommend you use `example_config.toml` as the base or just copy it. `example_config.toml` uses vim key bindings.
