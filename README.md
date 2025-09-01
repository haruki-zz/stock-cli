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
stock-cli interactive
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

**Available Options:**
- **Update Stock Data**: Fetch fresh stock information from API
- **Show Stock Info**: Display information for specific stock codes (you'll be prompted to enter codes)
- **Filter Stocks**: Show stocks matching configured thresholds
- **Load from File**: Load stock data from CSV file (you'll be prompted to enter filename)
- **Exit**: Exit the application

The interface is designed similar to Claude CLI with a clean menu system and intuitive navigation.

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
cargo run -- interactive

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy
```

## License

For personal use only. Please respect the original author's license terms.