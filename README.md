# upyremote

Rust CLI tool for interacting with MicroPython devices, inspired by mpremote.

## Features

- **Interactive REPL Connection**: Connect directly to MicroPython REPL with support for history and line editing
- **File Transfer**: Upload and download files using base64 encoding
- **Command Execution**: Execute Python commands remotely
- **Device Management**: Soft and hard reset of the device
- **Script Mode**: Compatible with pipes and redirection
- **Cross-platform**: Works on Linux, macOS, and Windows

## Installation

### From Source

```bash
git clone <repository-url>
cd upyremote
cargo build --release
```

The compiled binary will be at `./target/release/upyremote`

### Requirements

- Rust 1.70 or higher
- Serial port access (usually requires belonging to the `dialout` group on Linux)

## Usage

### Available Commands

#### `connect` - Interactive REPL Connection

```bash
upyremote connect -p /dev/ttyACM0
```

Opens an interactive REPL session with the device. Press `Ctrl+X` to exit.

**Keyboard shortcuts in REPL mode:**

| Shortcut | Action |
|----------|--------|
| `Ctrl+X` | Exit REPL |
| `Ctrl+C` | Interrupt running program |
| `Ctrl+D` | EOF / Soft reset |
| `Ctrl+A` | Go to beginning of line |
| `Ctrl+E` | Go to end of line |
| `Ctrl+K` | Delete to end of line |
| `Ctrl+U` | Delete entire line |
| `Ctrl+W` | Delete previous word |
| `Ctrl+←` | Jump word backward |
| `Ctrl+→` | Jump word forward |
| `↑` / `↓` | Navigate command history |
| `←` / `→` | Move cursor |
| `Home` / `End` | Go to beginning/end of line |
| `Delete` | Delete character under cursor |

#### `ls` - List Files

```bash
upyremote ls -p /dev/ttyACM0 /path/directory
```

Lists files in the specified directory on the device.

#### `put` - Upload File

```bash
upyremote put -p /dev/ttyACM0 local_file.py /remote/path/file.py
```

Uploads a local file to the device. If destination is not specified, uses the local filename.

#### `get` - Download File

```bash
upyremote get -p /dev/ttyACM0 /remote/path/file.py local_file.py
```

Downloads a file from the device. If local destination is not specified, uses the remote filename.

#### `exec` - Execute Python Command

```bash
upyremote exec -p /dev/ttyACM0 "print('Hello World')"
upyremote exec -p /dev/ttyACM0 "import os; print(os.listdir('/'))"
```

Executes Python code on the device using the raw REPL protocol.

#### `run` - Run Python File

```bash
upyremote run -p /dev/ttyACM0 script.py
```

Reads a local Python file and executes it on the device.

#### `send` - Send Text String

```bash
# Automatically waits for prompt (>>> or $:)
upyremote send -p /dev/ttyACM0 "print('Hello')"

# With specific timeout (in seconds)
upyremote send -p /dev/ttyACM0 "command" -t 5
```

Sends a text string directly to the serial port and displays the response.
- Without `-t`: Waits until the device prompt is received
- With `-t`: Reads for the specified duration

#### `reset` - Reset Device

```bash
# Soft reset (Ctrl+D in MicroPython)
upyremote reset -p /dev/ttyACM0

# Hard reset (toggles DTR/RTS signals)
upyremote reset -p /dev/ttyACM0 -H
```

## Usage Examples

### Basic Connection

```bash
# Connect to REPL
upyremote connect -p /dev/ttyACM0

# In the REPL, you can use:
# >>> print("Hello")
# >>> import os
# >>> os.listdir('/')
```

### File Management

```bash
# Upload a script
upyremote put -p /dev/ttyACM0 main.py

# Download a log file
upyremote get -p /dev/ttyACM0 /log.txt backup_log.txt

# View files in root directory
upyremote ls -p /dev/ttyACM0 /
```

### Command Execution

```bash
# Execute simple code
upyremote exec -p /dev/ttyACM0 "print(2+2)"

# View system information
upyremote exec -p /dev/ttyACM0 "import sys; print(sys.version)"

# List files
upyremote exec -p /dev/ttyACM0 "import os; print(os.listdir('/'))"
```

### Script Usage

```bash
# Send multiple commands
echo -e "x = 100\nprint(x)" | upyremote connect -p /dev/ttyACM0

# Automate tasks
upyremote send -p /dev/ttyACM0 "import machine; machine.freq()" -t 2
```

## Global Options

Each command accepts the following options:

- `-p, --port <PORT>`: Serial port (default: `/dev/ttyUSB0`)
  - Linux: `/dev/ttyUSB0`, `/dev/ttyACM0`
  - macOS: `/dev/cu.usbserial*`, `/dev/cu.usbmodem*`
  - Windows: `COM3`, `COM4`, etc.

## Troubleshooting

### Permission denied accessing port

On Linux, add your user to the `dialout` group:

```bash
sudo usermod -a -G dialout $USER
# Log out and log back in
```

### Device not found

Verify that the device is connected:

```bash
# Linux
ls -la /dev/ttyACM* /dev/ttyUSB*

# macOS
ls -la /dev/cu.*
```

### Port busy

If you receive "Device or resource busy", check that no other process is using the port:

```bash
lsof /dev/ttyACM0
# or
fuser /dev/ttyACM0
```

## Development

### Compile in debug mode

```bash
cargo build
```

### Compile in release mode (optimized)

```bash
cargo build --release
```

### Run tests

```bash
cargo test
```

## Architecture

The project uses:
- **clap**: Command line argument parser
- **serialport**: Cross-platform serial communication
- **crossterm**: Raw terminal handling for interactive REPL
- **anyhow**: Error handling

## License

MIT License - See LICENSE for details.

## Contributing

Contributions are welcome. Please open an issue or pull request.

## Acknowledgments

Inspired by [mpremote](https://docs.micropython.org/en/latest/reference/mpremote.html), the official MicroPython tool.
