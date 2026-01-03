# local search
 - Fully local search engine written in Rust, with native ui via raylib (web ewwww).

## Build
You might need raylib globally or other dependencies I honestly can't tell since I could build it 
out of the box. Otherwise just:
```bash
    cargo build --release
```
Build release trust me the performance otherwise is subpar.

## Configuration
The configuration is a .toml format file, 
that's in ~/.config/local-search/config.toml for linux and C:\Users\%USERNAME%\AppData\Roaming\local-search/config.toml for windows (niche video game OS).
It's created on first start with the needed keys.
```rust
struct Config {
    document_directories: Vec<String>,
    font_name: Option<String>,
    theme: Theme,
}
```
Example .toml config:
```toml
document_directories = ["/home/issac/Documents/local-search"]
theme = "default"
```
By default a search path for your Documents/local-search is appended to the document_directories key.


## Keybinds (not customizable *yet*!):
 - <C-d> show debug info
 - <r> (while not focused on the query input box) reindex the files (blocks the UI)
 - <Enter> do query


## Built-in themes (*PR's are open for more!*)
 - Catppuccin Latte/Mocha (theme = "catppuccin-mocha")
 - Default (theme = "default", or unspecified)

## Misc. info
 - The index file is stored in ~/.local/state/local-search/index.bin (linux) or C:\Users\%USERNAME%\AppData\Local\local-search\index.bin (windows)
 - Uses tf-idf
 - Press on a result document to open it (via xdg-open or other OS specific protocol)

## TODO
 - Todos are in the source files
