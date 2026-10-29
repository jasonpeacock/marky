# Marky

Markdown Magician 🧙

**Features**

- Hot reload previewing 🔥
- Conversion to **HTML**  🏭
- Themes! ✨
- Extensions - Math, diagrams, syntax-highlighting 🧩
- Download base64 encoded images (png, jpg, svg)

<!--toc:start-->

- [Examples](#examples)
- [Install](#install)
- [Configuration](#configuration)
- [Help](#help)
- [Build](#build)
- [Screenshots](#screenshots)

<!--toc:end-->

## Examples

Convert `doc.md` to `doc.html`

```bash
marky doc.md
```

Start a local preview server with hot-reload

```bash
marky doc.md --live
```

Enable extensions

```bash
# Or use --all to enable all
marky doc.md --math --diagrams --highlight
```

Include local images as base64 encoded and compress them (beta)

```bash
# possible values: local, remote, all
marky doc.md --include-images "local" --optimize-images
# or short
marky doc.md -zI local
```

Select and use a different theme with fzf

```bash
marky doc.md --theme $(marky --themes | fzf)
```

Pipe from stdout and open compiled file

```bash
cat doc.md | marky --out doc.html --open
```

> See `--help` for more info

## Install

Install using [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)

```bash
cargo install marky
```

## Configuration

marky reads its config files from the directory that `marky --where-config` prints.
That is `~/.config/marky` on Linux and `~/Library/Application Support/marky` on macOS.
Both files are optional.

### config.toml

`config.toml` sets your default options, so you do not have to pass the same flags every time.
Flags on the command line take precedence:

- `--theme` and `--port` replace the value from the config file.
- Boolean flags such as `--math` can only turn an option on. If the config file
  turns an option on, no flag turns it off for a single run.

This example shows every key with its default value. Copy it into
`config.toml` and change the keys you need. You can leave out any key.

```toml
# Theme to use. See `marky --themes` for the available names.
theme = "sakura"

# Enable syntax highlighting with highlight.js (--highlight).
highlight = false

# Enable math rendering with KaTeX (--math).
math = false

# Enable UML diagram rendering with Mermaid (--diagrams).
diagrams = false

# Include images into the file as base64. Not set by default, so images are
# left as links. Possible values: "local", "remote", "all" (--include-images).
# include_images = "local"

# Optimize included images to make them smaller (--optimize-images).
optimize_images = false

# Port of the live server (--port).
port = 8080

# Open the output file in the default app (--open).
open = false
```

marky stops with an error if `config.toml` has an unknown key or a value of the wrong type.

### themes.toml

`themes.toml` adds your own themes to the built-in ones. Each `[[themes]]`
entry needs a `name` and one source of CSS:

- `path` is a CSS file. A relative path is relative to the config directory.
- `inline` is the CSS itself.

```toml
[[themes]]
name = "mytheme"
path = "themes/mytheme.css"

[[themes]]
name = "tiny"
inline = """
body { max-width: 40em; margin: auto; font-family: sans-serif; }
"""
```

Then use the theme by name, on the command line or in `config.toml`:

```bash
marky doc.md --theme mytheme
```

## Help

```
Markdown Magician 🧙

Usage: marky [OPTIONS] [PATH]

Arguments:
  [PATH]  Read input from file

Options:
      --completion <GENERATOR>
          [possible values: bash, elvish, fish, powershell, zsh]
  -t, --theme <THEME>
          Theme to use
      --string <STRING>
          Read input from string
      --themes
          List available themes
      --where-config
          Print config path
  -o, --out <OUT>
          Output file
      --stdout
          Output to stdout
  -H, --highlight
          Enable syntax highligting with highlight.js
  -M, --math
          Enable math rendering with KaTeX
  -D, --diagrams
          Enable UML diagrams rendering with Mermaid
  -I, --include-images <INCLUDE_IMAGES>
          Include images into file as base64 encoded [possible values: local, remote, all]
  -z, --optimize-images
          Optimize included images to make them smaller
  -A, --all
          Enable all extra renderers
  -w, --watch
          Recompile file on save
  -l, --live
          Live preview in the browser
      --port <PORT>
          Port of the live server [default: 8080]
  -O, --open
          Open output file in the default app
  -h, --help
          Print help
  -V, --version
          Print version
```

## Build

```bash
git clone https://github.com/metafates/marky.git
cd marky
cargo install --path .
```

## Screenshots

Some examples...

```bash
marky README.md --theme sakura # default theme
```

![sakura](https://user-images.githubusercontent.com/62389790/216391306-ecd73229-6342-4a79-8f7f-5f632a231a6f.png)

```bash
marky README.md --theme air
```

![air](https://user-images.githubusercontent.com/62389790/216391415-46ca090a-801d-423e-a523-dc3e59ed1f77.png)

```bash
marky README.md --theme retro
```

![retro](https://user-images.githubusercontent.com/62389790/216391465-ddfff1ad-3cd6-43b8-a193-fc9c664ec018.png)

See `marky --themes` to show all available themes.

You can also add your own themes, see [themes.toml](#themestoml).
