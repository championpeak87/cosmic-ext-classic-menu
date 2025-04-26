# cosmic-classic-menu

Cosmic Classic Menu is a customizable application launcher for the Cosmic desktop environment. It provides a classic-style menu for launching applications, accessing system tools, and managing power options.

![COSMIC Classic Menu Screenshot](cosmic-classic-menu.png)

## Features

- Classic-style application menu
- Search functionality with fuzzy matching and typo tolerance
- Categorized application list
- Recently used applications
- Power options (shutdown, restart, logout, etc.)
- System tools (settings, system monitor, disk management)

## Installation 

Clone the repository:

```bash
git clone https://github.com/championpeak87/cosmic-classic-menu cosmic-classic-menu
cd cosmic-classic-menu
```

Build and install the project:

```bash
just build-release
sudo just install
```

For alternative packaging methods, use the one of the following recipes:

- `deb`: run `just build-deb` and `sudo just install-deb`
- `rpm`: run `just build-rpm` and `sudo just install-rpm`

For vendoring, use `just vendor` and `just vendor-build`

## Contributing

A [justfile](./justfile) is included with common recipes used by other COSMIC projects:

- `just build-debug` compiles with debug profile
- `just run` builds and runs the application
- `just check` runs clippy on the project to check for linter warnings
- `just check-json` can be used by IDEs that support LSP

## License

Code is distributed with the [GPL-3.0-only license][./LICENSE]

