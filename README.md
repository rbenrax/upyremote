# upyremote

Rust CLI tool for interacting with MicroPython devices, inspired by mpremote. Supports both standard MicroPython REPL and [upyOS](https://github.com/rbenrax/upyOS) (POSIX-like OS for microcontrollers).

## Features

- **Automatic Mode Detection**: Automatically detects if device is in MicroPython REPL or upyOS mode
- **Interactive REPL Connection**: Connect directly to device with support for history and line editing
- **File Transfer**: Upload and download files (base64 for REPL, direct transfer for upyOS)
- **Command Execution**: Execute Python commands in REPL mode, shell commands in upyOS mode
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

## Device Modes

upyremote automatically detects the operating mode of your MicroPython device:

### MicroPython REPL Mode
Standard MicroPython interactive prompt (`>>>`).
- Uses raw REPL protocol for file transfers
- Supports Python code execution
- Base64 encoding for binary file transfers

### upyOS Mode
[upyOS](https://github.com/rbenrax/upyOS) provides a POSIX-like shell environment (`/ $:`).
- Linux-like shell commands
- Direct file operations using shell commands
- Support for upyOS-specific features

Upon connection, upyremote displays the detected mode:
```
[INFO] Detected mode: upyOS (Linux-like shell)
```

## Usage

### Available Commands by Mode

| Command | MicroPython REPL | upyOS | Description |
|---------|------------------|-------|-------------|
| `connect` | ✓ | ✓ | Interactive REPL/shell session |
| `ls` | ✓ | ✓ | List files |
| `put` | ✓ | ✓ | Upload file |
| `get` | ✓ | ✓ | Download file |
| `send` | ✓ | ✓ | Send raw string |
| `reset` | ✓ | ✓ | Reset device |
| `exec` | ✓ | ✗ | Execute Python code |
| `run` | ✓ | ✗ | Run Python file |

### Commands

#### `connect` - Interactive Connection

```bash
upyremote connect -p /dev/ttyACM0
```

Opens an interactive session. Mode-appropriate prompt is displayed:
- MicroPython: `MicroPython REPL ---`
- upyOS: `upyOS Shell ---`

Press `Ctrl+X` to exit.

**Keyboard shortcuts:**

| Shortcut | Action |
|----------|--------|
| `Ctrl+X` | Exit session |
| `Ctrl+C` | Interrupt program |
| `Ctrl+D` | EOF / Soft reset |
| `Ctrl+A` | Beginning of line |
| `Ctrl+E` | End of line |
| `Ctrl+K` | Delete to end of line |
| `Ctrl+U` | Delete entire line |
| `Ctrl+W` | Delete previous word |
| `Ctrl+←/→` | Jump word by word |
| `↑/↓` | Command history |

#### `ls` - List Files

Works in both modes automatically.

```bash
upyremote ls -p /dev/ttyACM0 /path/directory
```

#### `put` - Upload File

Automatically adapts transfer method based on detected mode.

```bash
# Upload to current directory
upyremote put -p /dev/ttyACM0 local_file.py

# Upload to specific path
upyremote put -p /dev/ttyACM0 local_file.py /remote/path/file.py

# Upload to upyOS specific location
upyremote put -p /dev/ttyACM0 script.sh /bin/myscript
```

**MicroPython REPL mode:** Uses base64 encoding via raw REPL protocol
**upyOS mode:** Uses `cat` command with heredoc

#### `get` - Download File

Automatically adapts transfer method based on detected mode.

```bash
# Download to current directory
upyremote get -p /dev/ttyACM0 /remote/file.py

# Download to specific path
upyremote get -p /dev/ttyACM0 /remote/file.py local_backup.py
```

**MicroPython REPL mode:** Uses base64 decoding via raw REPL protocol
**upyOS mode:** Uses `cat` command

#### `exec` - Execute Python Command

Only available in MicroPython REPL mode.

```bash
upyremote exec -p /dev/ttyACM0 "print('Hello World')"
upyremote exec -p /dev/ttyACM0 "import os; print(os.listdir('/'))"
```

**Note:** Will display error if device is in upyOS mode.

#### `run` - Run Python File

Only available in MicroPython REPL mode.

```bash
upyremote run -p /dev/ttyACM0 script.py
```

**Note:** Will display error if device is in upyOS mode.

#### `send` - Send Text String

Works in both modes. Universal command for sending raw strings.

```bash
# Auto-detects prompt (waits for >>> or $:)
upyremote send -p /dev/ttyACM0 "print('Hello')"

# With timeout (for commands that take time)
upyremote send -p /dev/ttyACM0 "wifi sta scan" -t 5

# upyOS shell command
upyremote send -p /dev/ttyACM0 "ps"
```

Options:
- Without `-t`: Waits for device prompt
- With `-t`: Reads for specified seconds

#### `reset` - Reset Device

Works in both modes.

```bash
# Soft reset (Ctrl+D in MicroPython)
upyremote reset -p /dev/ttyACM0

# Hard reset (DTR/RTS toggle)
upyremote reset -p /dev/ttyACM0 -H
```

## Usage Examples

### MicroPython REPL Mode

```bash
# Connect and work interactively
upyremote connect -p /dev/ttyACM0
# >>> print("Hello from MicroPython")

# Execute Python code
upyremote exec -p /dev/ttyACM0 "print(2+2)"

# Upload a script
upyremote put -p /dev/ttyACM0 main.py

# Run a script
upyremote run -p /dev/ttyACM0 sensor.py
```

### upyOS Mode

```bash
# Connect to upyOS shell
upyremote connect -p /dev/ttyACM0
# / $: ls

# List files
upyremote ls -p /dev/ttyACM0 /

# Upload a script to upyOS
upyremote put -p /dev/ttyACM0 mi_script.sh /bin/mi_script

# Execute upyOS command
upyremote send -p /dev/ttyACM0 "wifi sta status"

# View running processes
upyremote send -p /dev/ttyACM0 "ps"

# Check system info
upyremote send -p /dev/ttyACM0 "lshw"
```

### Mixed Mode Workflow

```bash
# Device boots into upyOS
# Upload a Python script
upyremote put -p /dev/ttyACM0 app.py /

# Execute it in upyOS
upyremote send -p /dev/ttyACM0 "python app.py &"

# Check it's running
upyremote send -p /dev/ttyACM0 "ps"

# Soft reset to enter MicroPython REPL mode
upyremote reset -p /dev/ttyACM0

# Now exec works
upyremote exec -p /dev/ttyACM0 "print('Now in REPL mode')"
```

## Global Options

- `-p, --port <PORT>`: Serial port
  - Priority order: Explicit argument > Environment variable > Default
  - Environment variable: `UPYREMOTE_PORT`
  - Default: `/dev/ttyACM0`
  - Linux: `/dev/ttyACM0`, `/dev/ttyUSB0`
  - macOS: `/dev/cu.usbserial*`, `/dev/cu.usbmodem*`
  - Windows: `COM3`, `COM4`, etc.

### Using Environment Variable

You can set the `UPYREMOTE_PORT` environment variable to avoid specifying the port every time:

```bash
# Set the port for the current session
export UPYREMOTE_PORT=/dev/ttyUSB0

# Now all commands use this port by default
upyremote ls
upyremote put main.py
upyremote get /data/log.txt

# You can still override with -p for specific commands
upyremote ls -p /dev/ttyACM0
```

Priority order:
1. Explicit `-p` argument (highest priority)
2. `UPYREMOTE_PORT` environment variable
3. Default `/dev/ttyACM0` (lowest priority)

## Troubleshooting

### Mode Detection Failed

If mode detection fails, some commands may not work properly. Try:

```bash
# Send a simple command to verify connectivity
upyremote send -p /dev/ttyACM0 "help" -t 2
```

### Command Not Available in Current Mode

Error example:
```
Error: This command requires MicroPython REPL mode, but device is in upyOS (Linux-like shell) mode.
Use 'upyremote send' command for upyOS operations or restart device to MicroPython mode.
```

**Solution:** Use `send` command for upyOS operations or reset device to switch modes.

### Permission Denied

On Linux, add your user to the `dialout` group:

```bash
sudo usermod -a -G dialout $USER
# Log out and log back in
```

### Port Busy

Check for other processes using the port:

```bash
lsof /dev/ttyACM0
fuser /dev/ttyACM0
```

## Development

### Compile

```bash
# Debug mode
cargo build

# Release mode (optimized)
cargo build --release
```

### Run Tests

```bash
cargo test
```

## Architecture

- **clap**: Command line argument parser
- **serialport**: Cross-platform serial communication
- **crossterm**: Raw terminal handling for interactive mode
- **anyhow**: Error handling

## Related Projects

- [mpremote](https://docs.micropython.org/en/latest/reference/mpremote.html): Official MicroPython tool
- [upyOS](https://github.com/rbenrax/upyOS): POSIX-like OS for microcontrollers

## License

MIT License - See LICENSE for details.

## Contributing

Contributions are welcome! Please open an issue or pull request.
