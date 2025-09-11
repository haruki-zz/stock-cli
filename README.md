# Stock CLI - Rust Edition

A high-performance CLI application for fetching and analyzing Chinese A-share stock information, rewritten from Python to Rust.

## Features

- **Async Stock Data Fetching**: Concurrent HTTP requests with configurable rate limiting
- **Interactive Menu Navigation**: Arrow key navigation similar to Claude CLI
- **Data Persistence**: Automatic CSV storage with timestamp-based file naming
- **Stock Filtering**: Filter stocks based on configurable thresholds (turnover, increase rate, etc.)
- **Progress Tracking**: Real-time progress indicators during data fetching
- **Unicode Support**: Proper handling of Chinese characters in stock names
- **Error Handling**: Robust error handling with retry mechanisms

## Installation

### Option 1: Build from Source
```bash
# Clone or download the project
cd stock-cli

# Build the project
cargo build --release

# The binary will be available at target/release/stock-cli
```

### Option 2: Easy Deployment
```bash
# Deploy to ~/bin (or specify custom directory)
./deploy.sh

# Or deploy to custom location
./deploy.sh /usr/local/bin
```

The deployment script will copy:
- The compiled binary (`stock-cli`)
- Configuration file (`config.json`)  
- Sample stock codes file (`stock_code.csv`)

### Option 3: Standalone Binary
The application is designed to work without external files:
- **Config file**: Searches in binary directory, current directory, then fails
- **Stock codes**: Uses built-in defaults if file doesn't exist
- **Data files**: Created automatically in current directory

```bash
# Copy just the binary anywhere
cp target/release/stock-cli /usr/local/bin/
cp config.json /usr/local/bin/  # Required

# Now works from any directory
stock-cli
```

## Configuration

The application uses `config.json` for configuration. The file contains:
- API endpoints and headers
- Stock information field mappings
- Filtering thresholds for different metrics
- Regional configurations

## Usage

### Interactive Mode (Only Mode)
```bash
./stock-cli
```

The application starts in interactive mode with arrow key navigation:

**Navigation:**
- **↑/↓ Arrow Keys**: Navigate between menu options
- **Enter**: Select the highlighted option
- **Esc** or **Ctrl+C**: Exit the application

**Available Options (in order):**
- **Filter Stocks**: List stocks that meet the current thresholds
- **Edit Thresholds**: Change the numeric ranges used for filtering
- **Refresh Data**: Fetch the latest stock data from the API and save
- **View Stocks**: Display info for stock codes you enter
- **Load CSV**: Load previously saved stock data from a CSV file
- **Quit**: Exit the application

Startup behavior and layout:
- The app opens in an alternate screen with the main menu fixed at the top.
- The banner shows the currently loaded data file: “Loaded data file: <name or None>”.
- Prompts and results render below the menu; press Enter to return to the menu.

### Edit Thresholds

- Open via: Menu → Edit Thresholds.
- Navigation: Up/Down arrows move the selection; selection wraps around from last to first.
- Edit: Press Enter on a metric to change its lower/upper bounds; values are shown and entered as two decimals.
- Add: Choose “Add new metric” to create a custom metric with bounds.
- Exit: Choose “Done” or press Esc.
- Scope: Changes apply in-memory for the current session. To make them default, edit `config.json`.

## Stock Codes File

Create a `stock_code.csv` file with one stock code per line:
```
sh600000
sz000001
sh600036
sz000002
```

If the file doesn't exist, the application will create a sample file with common stock codes.

## Data Format

The application fetches and stores the following stock information:
- Stock Name
- Stock Code  
- Current Price
- Previous Close
- Open Price
- Increase/Decrease
- Highest Price
- Lowest Price
- Turnover Rate
- Amplitude
- Market Value

## Performance

The Rust version offers significant performance improvements over the Python original:
- **Faster startup time**: No Python interpreter overhead
- **Better concurrency**: Tokio async runtime with configurable semaphores
- **Lower memory usage**: Efficient memory management
- **Better error handling**: Type-safe error handling with anyhow

## Development

```bash
# Run in development mode
cargo run

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy
```

## License

For personal use only. Please respect the original author's license terms.
