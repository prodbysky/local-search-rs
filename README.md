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
    background_color: Option<Color>,
    foreground_color: Option<Color>,

    idle_color: Option<Color>,
    hovered_color: Option<Color>,
    clicked_color: Option<Color>,
}
```
By default a search path for your Documents/local-search is appended to the document_directories key.
Colors are RGB tables.

### Key descriptions:
 - document_directories: directories in which to find files to index (recursively)
 - font_name           : fc-list compatible name of a font (if not provided will use builtin GeistMono)
 - others should be self explanatory :)

## Misc. info
 - The index file is stored in ~/.local/state/local-search/index.json (linux) or C:\Users\%USERNAME%\AppData\Local\local-search\index.json (windows)
 - Uses tf-idf

## TODO
 - Todos are in the source files
