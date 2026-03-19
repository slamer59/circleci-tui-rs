# CircleCI TUI

> ⚠️ **PROOF OF CONCEPT** - This project is in early development and is not production-ready. Use at your own risk.

A fast, keyboard-driven terminal user interface (TUI) for monitoring and interacting with CircleCI pipelines, workflows, and jobs.

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![CircleCI](https://img.shields.io/badge/circle%20ci-%23161616.svg?style=for-the-badge&logo=circleci&logoColor=white)

## Features

- 📊 **Pipeline Overview** - View recent pipelines for your project at a glance
- 🔄 **Workflow Details** - Drill down into workflows and see their execution status
- 🎯 **Job Monitoring** - Track individual job progress and execution times
- 📝 **Live Logs** - Stream job logs in real-time with automatic updates
- ⌨️ **Keyboard-Driven** - Navigate efficiently with Vim-style keybindings
- 🎨 **Customizable** - Theme support and user preferences
- 📋 **Log Selection** - Copy specific line ranges with Vim-style syntax
- ⚡ **Fast & Lightweight** - Built with Rust for optimal performance
- 🗄️ **Smart Caching** - Local cache with configurable retention

## Prerequisites

- Rust 1.70 or later
- CircleCI API token
- Git (for repository detection)

## Installation

### Option 1: Install directly from GitHub

```bash
cargo install --git https://github.com/slamer59/circleci-tui-rs
```

To install a specific branch:
```bash
cargo install --git https://github.com/slamer59/circleci-tui-rs --branch main
```

After installation, run:
```bash
circleci-tui
```

### Option 2: Clone and build locally

```bash
# Clone the repository
git clone https://github.com/slamer59/circleci-tui-rs.git
cd circleci-tui-rs

# Build and run
cargo run --release
```

Or with a specific manifest:
```bash
cargo run --manifest-path ./Cargo.toml --release
```

### Option 3: Build and install from local source

```bash
# From the project directory
cargo install --path .
```

## Configuration

Create a `.env` file in your current working directory or set environment variables:

```env
CIRCLECI_TOKEN=your_circleci_api_token_here
PROJECT_SLUG=gh/owner/repo
```

### Getting a CircleCI Token

1. Go to [CircleCI User Settings](https://app.circleci.com/settings/user/tokens)
2. Create a new Personal API Token
3. Copy the token to your `.env` file

### Finding Your Project Slug

Your project slug follows the format: `<vcs>/<org>/<repo>`
- For GitHub: `gh/your-username/your-repo`
- For Bitbucket: `bb/your-username/your-repo`

## Usage

### Keyboard Shortcuts

#### Navigation
- `j` / `↓` - Move down
- `k` / `↑` - Move up
- `Enter` - Select/drill down into item
- `Esc` / `Backspace` - Go back/close modal
- `q` - Quit application

#### Actions
- `r` - Refresh current view
- `c` - Copy logs (when viewing job logs)
- `/` - Search (where applicable)

#### Log Copy Selection
When viewing job logs, press `c` to copy lines:
- Single line: `42`
- Range: `10-20`
- From line to end: `30-`
- From start to line: `-15`

## Project Structure

```
circleci-tui-rs/
├── src/
│   ├── api/          # CircleCI API client
│   ├── cache/        # Local caching implementation
│   ├── ui/           # UI components and screens
│   ├── app.rs        # Main application logic
│   ├── config.rs     # Configuration management
│   ├── events.rs     # Event handling
│   ├── git.rs        # Git integration
│   ├── preferences.rs # User preferences
│   ├── theme.rs      # Theme system
│   └── main.rs       # Entry point
├── tests/            # Integration tests
└── Cargo.toml        # Project dependencies
```

## Architecture

Built with:
- **[ratatui](https://github.com/ratatui/ratatui)** - Terminal UI framework
- **[crossterm](https://github.com/crossterm-rs/crossterm)** - Cross-platform terminal manipulation
- **[tokio](https://tokio.rs/)** - Async runtime for concurrent operations
- **[reqwest](https://github.com/seanmonstar/reqwest)** - HTTP client for CircleCI API
- **[serde](https://serde.rs/)** - Serialization/deserialization

## Development

### Running Tests

```bash
cargo test
```

### Running with Debug Logs

Uncomment the tracing dependencies in `Cargo.toml` and run:

```bash
RUST_LOG=debug cargo run
```

### Building for Release

```bash
cargo build --release
```

## Contributing

Contributions are welcome! This is an early-stage project and there's plenty of room for improvement.

### How to Contribute

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Ideas for Contributions

- [ ] Add support for re-running failed workflows/jobs
- [ ] Implement filtering and advanced search
- [ ] Add more customization options
- [ ] Improve error handling and user feedback
- [ ] Add comprehensive test coverage
- [ ] Support multiple projects
- [ ] Add GitHub Actions integration

## License

License has not been determined yet. This repository is currently private.

## Acknowledgments

- Built with ❤️ using [Ratatui](https://ratatui.rs/)
- Inspired by terminal UI tools like lazygit, k9s, and htop

## Support

If you encounter issues or have questions:
- Open an issue in this repository
- Check existing issues for similar problems

---

**Note**: This is a proof of concept project and may contain bugs or incomplete features. Use in production environments is not recommended at this time.
