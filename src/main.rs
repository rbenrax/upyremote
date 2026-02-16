use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use serialport::{DataBits, FlowControl, Parity, StopBits};
use std::{
    io::{self, Read, Write},
    path::PathBuf,
    thread,
    time::Duration,
};

#[derive(Parser)]
#[command(name = "upyremote")]
#[command(about = "CLI tool for interacting with MicroPython devices")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Connect to device and open interactive REPL
    Connect {
        /// Serial port (e.g., /dev/ttyUSB0 or COM3)
        #[arg(short, long, default_value = "/dev/ttyUSB0")]
        port: String,
        /// Baud rate
        #[arg(short, long, default_value = "115200")]
        baud: u32,
    },
    /// List files on device
    Ls {
        /// Serial port
        #[arg(short, long, default_value = "/dev/ttyUSB0")]
        port: String,
        /// Directory to list
        #[arg(default_value = "/")]
        path: String,
    },
    /// Upload a file to device
    Put {
        /// Serial port
        #[arg(short, long, default_value = "/dev/ttyUSB0")]
        port: String,
        /// Local file
        source: PathBuf,
        /// Destination on device (optional)
        dest: Option<String>,
    },
    /// Download a file from device
    Get {
        /// Serial port
        #[arg(short, long, default_value = "/dev/ttyUSB0")]
        port: String,
        /// File on device
        source: String,
        /// Local destination (optional)
        dest: Option<PathBuf>,
    },
    /// Execute a command on device
    Exec {
        /// Serial port
        #[arg(short, long, default_value = "/dev/ttyUSB0")]
        port: String,
        /// Command to execute
        command: String,
    },
    /// Reset device
    Reset {
        /// Serial port
        #[arg(short, long, default_value = "/dev/ttyUSB0")]
        port: String,
        /// Hard reset (complete reset)
        #[arg(short = 'H', long)]
        hard: bool,
    },
    /// Run a Python file on device
    Run {
        /// Serial port
        #[arg(short, long, default_value = "/dev/ttyUSB0")]
        port: String,
        /// File to run
        file: PathBuf,
    },
    /// Send a string to device and display response
    Send {
        /// Serial port
        #[arg(short, long, default_value = "/dev/ttyUSB0")]
        port: String,
        /// String to send
        data: String,
        /// Timeout in seconds for response (if not specified, waits for prompt " $: ")
        #[arg(short, long)]
        timeout: Option<u64>,
    },
}

struct MpDevice {
    port: Box<dyn serialport::SerialPort>,
}

impl MpDevice {
    fn new(port_name: &str, baud_rate: u32) -> Result<Self> {
        let port = serialport::new(port_name, baud_rate)
            .data_bits(DataBits::Eight)
            .parity(Parity::None)
            .stop_bits(StopBits::One)
            .flow_control(FlowControl::None)
            .timeout(Duration::from_millis(100))
            .open()
            .with_context(|| format!("Could not open port {}", port_name))?;

        Ok(MpDevice { port })
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        self.port.write_all(data)?;
        self.port.flush()?;
        Ok(())
    }

    fn read_available(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self.port.read(buf) {
            Ok(n) => Ok(n),
            Err(e) if e.kind() == io::ErrorKind::TimedOut => Ok(0),
            Err(e) => Err(e.into()),
        }
    }

    fn read_until(&mut self, needle: &[u8], buf: &mut Vec<u8>, timeout_ms: u64) -> Result<bool> {
        let start = std::time::Instant::now();
        let mut temp_buf = [0u8; 1024];

        loop {
            if start.elapsed().as_millis() > timeout_ms as u128 {
                return Ok(false);
            }

            match self.port.read(&mut temp_buf) {
                Ok(n) if n > 0 => {
                    buf.extend_from_slice(&temp_buf[..n]);
                    if buf.windows(needle.len()).any(|w| w == needle) {
                        return Ok(true);
                    }
                }
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::TimedOut => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    fn enter_raw_repl(&mut self) -> Result<()> {
        // Clear input buffer
        let mut discard = [0u8; 1024];
        let _ = self.port.read(&mut discard);

        // Ctrl-C to interrupt any running program
        self.write(&[0x03, 0x03])?;
        thread::sleep(Duration::from_millis(200));

        // Ctrl-A to enter raw REPL
        self.write(&[0x01])?;
        thread::sleep(Duration::from_millis(200));

        // Read prompt
        let mut buf = vec![];
        if self.read_until(b">>>", &mut buf, 1000)? {
            // Verify we are in raw mode
            let response = String::from_utf8_lossy(&buf);
            if response.contains("raw REPL") || response.contains("CTRL-B") {
                return Ok(());
            }
        }

        // Try again
        self.write(&[0x01])?;
        thread::sleep(Duration::from_millis(500));
        Ok(())
    }

    fn exit_raw_repl(&mut self) -> Result<()> {
        // Ctrl-B to exit raw REPL
        self.write(&[0x02])?;
        thread::sleep(Duration::from_millis(200));
        Ok(())
    }

    fn exec_command(&mut self, code: &str) -> Result<String> {
        self.enter_raw_repl()?;

        // Send code
        let code_bytes = code.as_bytes();

        // Send in chunks
        for chunk in code_bytes.chunks(256) {
            self.write(chunk)?;
            thread::sleep(Duration::from_millis(50));
        }

        // Ctrl-D to execute
        self.write(&[0x04])?;

        // Read response
        let mut response = vec![];
        self.read_until(b"\x04>", &mut response, 5000)?;

        self.exit_raw_repl()?;

        // Parse response
        let output = String::from_utf8_lossy(&response);

        // Look between OK and \x04 markers
        if let Some(start) = output.find("OK") {
            let rest = &output[start + 2..];
            if let Some(end) = rest.find('\x04') {
                let result = &rest[..end];
                // Clean output
                return Ok(result.trim().to_string());
            }
        }

        Ok(output.to_string())
    }

    fn list_files(&mut self, path: &str) -> Result<Vec<String>> {
        let cmd = format!(
            r#"import os
try:
    files = os.listdir("{}")
    for f in files:
        print(f)
except OSError as e:
    print("Error:", e)"#,
            path
        );

        let output = self.exec_command(&cmd)?;
        let files: Vec<String> = output
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && !s.starts_with("Error:"))
            .collect();

        Ok(files)
    }

    fn put_file(&mut self, local_path: &PathBuf, remote_path: &str) -> Result<()> {
        let content = std::fs::read(local_path)
            .with_context(|| format!("Could not read {}", local_path.display()))?;

        // Check size
        if content.len() > 10000 {
            println!(
                "Large file ({} bytes), uploading in parts...",
                content.len()
            );
        }

        // Encode content in base64
        let b64_content = base64_encode(&content);

        let cmd = format!(
            r#"import ubinascii
data = ubinascii.a2b_base64('{}')
with open('{}', 'wb') as f:
    f.write(data)
print('OK')"#,
            b64_content, remote_path
        );

        let result = self.exec_command(&cmd)?;

        if result.contains("OK") || result.is_empty() || result.lines().any(|l| l.contains("OK")) {
            println!(
                "✓ File '{}' uploaded to '{}' ({} bytes)",
                local_path.display(),
                remote_path,
                content.len()
            );
            Ok(())
        } else {
            anyhow::bail!("Error uploading file: {}", result)
        }
    }

    fn get_file(&mut self, remote_path: &str, local_path: &PathBuf) -> Result<()> {
        let cmd = format!(
            r#"import ubinascii
try:
    with open('{}', 'rb') as f:
        data = f.read()
        print(ubinascii.b2a_base64(data).decode().strip())
except OSError as e:
    print('Error:', e)"#,
            remote_path
        );

        let output = self.exec_command(&cmd)?;

        if output.contains("Error:") {
            anyhow::bail!("Error reading remote file: {}", output);
        }

        // Extract base64 from output
        let b64_data: String = output
            .lines()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && !s.contains(">>>") && !s.contains("OK"))
            .collect();

        if b64_data.is_empty() {
            anyhow::bail!("Could not read file '{}'", remote_path);
        }

        let content = base64_decode(&b64_data)?;
        let content_len = content.len();
        std::fs::write(local_path, &content)
            .with_context(|| format!("Could not write {}", local_path.display()))?;

        println!(
            "✓ File '{}' downloaded to '{}' ({} bytes)",
            remote_path,
            local_path.display(),
            content_len
        );
        Ok(())
    }

    fn soft_reset(&mut self) -> Result<()> {
        // Ctrl-D performs soft reset in MicroPython
        self.write(&[0x04])?;
        thread::sleep(Duration::from_millis(1000));
        println!("✓ Soft reset performed");
        Ok(())
    }

    fn hard_reset(&mut self) -> Result<()> {
        // Toggle DTR/RTS for hard reset on many ESP32 boards
        println!("Performing hard reset (DTR/RTS)...");
        self.port.write_data_terminal_ready(true)?;
        self.port.write_request_to_send(false)?;
        thread::sleep(Duration::from_millis(100));
        self.port.write_data_terminal_ready(false)?;
        self.port.write_request_to_send(true)?;
        thread::sleep(Duration::from_millis(100));
        self.port.write_request_to_send(false)?;
        thread::sleep(Duration::from_millis(1000));
        println!("✓ Hard reset performed");
        Ok(())
    }

    fn send_string(&mut self, data: &str, timeout_secs: Option<u64>) -> Result<String> {
        // Clear input buffer
        let mut discard = [0u8; 1024];
        let _ = self.port.read(&mut discard);

        // Send string
        self.write(data.as_bytes())?;

        // If doesn't end with newline, add it
        if !data.ends_with('\n') && !data.ends_with('\r') {
            self.write(b"\r")?;
        }

        // Read response
        let mut response = Vec::new();
        let mut buf = [0u8; 1024];
        let start = std::time::Instant::now();
        const LINUX_PROMPT: &[u8] = b" $: ";
        const MP_PROMPT: &[u8] = b">>>";
        const DEFAULT_TIMEOUT: u64 = 30; // 30 seconds max if timeout not specified

        let timeout = timeout_secs.unwrap_or(DEFAULT_TIMEOUT);
        let wait_for_prompt = timeout_secs.is_none();

        loop {
            // Check timeout
            if start.elapsed().as_secs() >= timeout {
                break;
            }

            match self.port.read(&mut buf) {
                Ok(n) if n > 0 => {
                    response.extend_from_slice(&buf[..n]);

                    // If waiting for prompt, check if we received one
                    if wait_for_prompt {
                        let has_linux_prompt = response
                            .windows(LINUX_PROMPT.len())
                            .any(|w| w == LINUX_PROMPT);
                        let has_mp_prompt =
                            response.windows(MP_PROMPT.len()).any(|w| w == MP_PROMPT);

                        if has_linux_prompt || has_mp_prompt {
                            // Give a bit more time in case there's more data
                            thread::sleep(Duration::from_millis(100));
                            // Try to read any additional data
                            let mut extra_buf = [0u8; 256];
                            if let Ok(n) = self.port.read(&mut extra_buf) {
                                if n > 0 {
                                    response.extend_from_slice(&extra_buf[..n]);
                                }
                            }
                            break;
                        }
                    }
                }
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::TimedOut => {
                    if !response.is_empty() && !wait_for_prompt {
                        // If we already received something and not waiting for prompt, give a bit more time
                        thread::sleep(Duration::from_millis(100));
                        // Check if there's more data
                        match self.port.read(&mut buf) {
                            Ok(n) if n > 0 => {
                                response.extend_from_slice(&buf[..n]);
                                continue;
                            }
                            _ => break,
                        }
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(e) => return Err(e.into()),
            }
        }

        let output = String::from_utf8_lossy(&response).to_string();
        Ok(output)
    }

    fn run_repl(&mut self) -> Result<()> {
        // Check if we are in an interactive terminal
        let is_tty = atty::is(atty::Stream::Stdin);

        if !is_tty {
            println!("Non-interactive mode detected. Using script mode.");
            println!("Type commands and press Ctrl+D to send, Ctrl+C to exit.");

            // Send Ctrl-C to interrupt any running program
            self.write(&[0x03])?;
            thread::sleep(Duration::from_millis(100));

            // Read any pending data
            let mut initial_buf = [0u8; 1024];
            if let Ok(n) = self.read_available(&mut initial_buf) {
                if n > 0 {
                    io::stdout().write_all(&initial_buf[..n])?;
                    io::stdout().flush()?;
                }
            }

            // Script mode: read lines from stdin
            let stdin = io::stdin();
            let mut stdout = io::stdout();
            let mut serial_buf = [0u8; 1024];
            let mut line = String::new();

            loop {
                // Read from serial port
                match self.read_available(&mut serial_buf) {
                    Ok(n) if n > 0 => {
                        stdout.write_all(&serial_buf[..n])?;
                        stdout.flush()?;
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }

                // Read from stdin (non-blocking)
                use std::io::BufRead;
                let mut stdin_lock = stdin.lock();
                if let Ok(n) = stdin_lock.read_line(&mut line) {
                    if n > 0 {
                        self.write(line.as_bytes())?;
                        self.write(b"\r")?;
                        line.clear();
                    }
                }

                thread::sleep(Duration::from_millis(10));
            }

            return Ok(());
        }

        // Interactive mode with raw terminal
        println!("Connected to device. Press Ctrl+X to exit.");
        println!("Use up/down arrows for command history.");
        println!("MicroPython REPL ---");
        println!();

        // Send Ctrl-C to interrupt any running program
        self.write(&[0x03])?;
        thread::sleep(Duration::from_millis(100));

        // Send Ctrl-B to ensure we are in normal mode (not raw)
        self.write(&[0x02])?;
        thread::sleep(Duration::from_millis(100));

        // Read any pending data
        let mut initial_buf = [0u8; 1024];
        if let Ok(n) = self.read_available(&mut initial_buf) {
            if n > 0 {
                io::stdout().write_all(&initial_buf[..n])?;
                io::stdout().flush()?;
            }
        }

        // Configure terminal
        if let Err(e) = enable_raw_mode() {
            eprintln!("Warning: Could not configure raw mode: {}", e);
            eprintln!("Continuing in line mode...");
        }

        let mut stdout = io::stdout();
        let mut serial_buf = [0u8; 1024];

        let result: Result<()> = (|| {
            let mut running = true;
            while running {
                // Read data from serial port (non-blocking)
                match self.read_available(&mut serial_buf) {
                    Ok(n) if n > 0 => {
                        stdout.write_all(&serial_buf[..n])?;
                        stdout.flush()?;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Error reading serial: {}", e);
                        break;
                    }
                }

                // Read user input
                if event::poll(Duration::from_millis(5))? {
                    if let Event::Key(key) = event::read()? {
                        match key.code {
                            // Ctrl+X to exit (before general Char case)
                            KeyCode::Char('x') | KeyCode::Char('X')
                                if key.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                running = false;
                            }
                            // Ctrl+C (interrupt)
                            KeyCode::Char('c') | KeyCode::Char('C')
                                if key.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                self.write(&[0x03])?;
                            }
                            // Ctrl+D (EOF/soft reset)
                            KeyCode::Char('d') | KeyCode::Char('D')
                                if key.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                self.write(&[0x04])?;
                            }
                            // Ctrl+A (beginning of line)
                            KeyCode::Char('a') | KeyCode::Char('A')
                                if key.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                self.write(&[0x01])?;
                            }
                            // Ctrl+E (end of line)
                            KeyCode::Char('e') | KeyCode::Char('E')
                                if key.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                self.write(&[0x05])?;
                            }
                            // Ctrl+K (delete to end of line)
                            KeyCode::Char('k') | KeyCode::Char('K')
                                if key.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                self.write(&[0x0b])?;
                            }
                            // Ctrl+U (delete entire line)
                            KeyCode::Char('u') | KeyCode::Char('U')
                                if key.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                self.write(&[0x15])?;
                            }
                            // Ctrl+W (delete previous word)
                            KeyCode::Char('w') | KeyCode::Char('W')
                                if key.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                self.write(&[0x17])?;
                            }
                            // Normal characters (including other controls)
                            KeyCode::Char(c) => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    // Send control characters (Ctrl+A = 0x01, etc.)
                                    let ctrl_char = (c as u8) & 0x1f;
                                    self.write(&[ctrl_char])?;
                                } else {
                                    self.write(&[c as u8])?;
                                }
                            }
                            // Enter
                            KeyCode::Enter => {
                                self.write(b"\r")?;
                            }
                            // Backspace
                            KeyCode::Backspace => {
                                self.write(&[0x7f])?;
                            }
                            // Tab
                            KeyCode::Tab => {
                                self.write(b"\t")?;
                            }
                            // Arrow Up - Previous history
                            KeyCode::Up => {
                                self.write(&[0x1b, 0x5b, 0x41])?;
                            }
                            // Arrow Down - Next history
                            KeyCode::Down => {
                                self.write(&[0x1b, 0x5b, 0x42])?;
                            }
                            // Arrow Right (Ctrl+Right = jump word forward)
                            KeyCode::Right => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    // Ctrl+Right: ESC[1;5C
                                    self.write(&[0x1b, 0x5b, 0x31, 0x3b, 0x35, 0x43])?;
                                } else {
                                    self.write(&[0x1b, 0x5b, 0x43])?;
                                }
                            }
                            // Arrow Left (Ctrl+Left = jump word backward)
                            KeyCode::Left => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    // Ctrl+Left: ESC[1;5D
                                    self.write(&[0x1b, 0x5b, 0x31, 0x3b, 0x35, 0x44])?;
                                } else {
                                    self.write(&[0x1b, 0x5b, 0x44])?;
                                }
                            }
                            // Home
                            KeyCode::Home => {
                                self.write(&[0x1b, 0x5b, 0x48])?;
                            }
                            // End
                            KeyCode::End => {
                                self.write(&[0x1b, 0x5b, 0x46])?;
                            }
                            // Delete
                            KeyCode::Delete => {
                                self.write(&[0x1b, 0x5b, 0x33, 0x7e])?;
                            }
                            // Escape
                            KeyCode::Esc => {
                                self.write(&[0x1b])?;
                            }
                            _ => {}
                        }
                    }
                }
            }
            Ok(())
        })();

        let _ = disable_raw_mode();
        println!("\nExiting REPL...");

        result
    }
}

// Simple base64 implementation
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);

    for chunk in data.chunks(3) {
        let b = match chunk.len() {
            1 => [chunk[0], 0, 0],
            2 => [chunk[0], chunk[1], 0],
            _ => [chunk[0], chunk[1], chunk[2]],
        };

        result.push(CHARS[(b[0] >> 2) as usize] as char);
        result.push(CHARS[(((b[0] & 0x03) << 4) | (b[1] >> 4)) as usize] as char);
        result.push(if chunk.len() > 1 {
            CHARS[(((b[1] & 0x0f) << 2) | (b[2] >> 6)) as usize] as char
        } else {
            '='
        });
        result.push(if chunk.len() > 2 {
            CHARS[(b[2] & 0x3f) as usize] as char
        } else {
            '='
        });
    }

    result
}

fn base64_decode(s: &str) -> Result<Vec<u8>> {
    let mut result = Vec::with_capacity(s.len() / 4 * 3);
    let chars: Vec<char> = s
        .chars()
        .filter(|&c| c != '=' && !c.is_whitespace())
        .collect();

    for chunk in chars.chunks(4) {
        if chunk.len() < 2 {
            continue;
        }

        let b0 = base64_char_value(chunk[0]);
        let b1 = base64_char_value(chunk[1]);

        result.push((b0 << 2) | (b1 >> 4));

        if chunk.len() > 2 {
            let b2 = base64_char_value(chunk[2]);
            result.push(((b1 & 0x0f) << 4) | (b2 >> 2));

            if chunk.len() > 3 {
                let b3 = base64_char_value(chunk[3]);
                result.push(((b2 & 0x03) << 6) | b3);
            }
        }
    }

    Ok(result)
}

fn base64_char_value(c: char) -> u8 {
    match c {
        'A'..='Z' => (c as u8) - b'A',
        'a'..='z' => (c as u8) - b'a' + 26,
        '0'..='9' => (c as u8) - b'0' + 52,
        '+' => 62,
        '/' => 63,
        _ => 0,
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Connect { port, baud } => {
            let mut device = MpDevice::new(&port, baud)?;
            device.run_repl()?;
        }
        Commands::Ls { port, path } => {
            let mut device = MpDevice::new(&port, 115200)?;
            let files = device.list_files(&path)?;
            println!("Files in '{}'", path);
            for file in files {
                println!("  {}", file);
            }
        }
        Commands::Put { port, source, dest } => {
            let mut device = MpDevice::new(&port, 115200)?;
            let remote_path = dest.unwrap_or_else(|| {
                source
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file.py")
                    .to_string()
            });
            device.put_file(&source, &remote_path)?;
        }
        Commands::Get { port, source, dest } => {
            let mut device = MpDevice::new(&port, 115200)?;
            let local_path = dest.unwrap_or_else(|| {
                PathBuf::from(
                    PathBuf::from(&source)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("download.py"),
                )
            });
            device.get_file(&source, &local_path)?;
        }
        Commands::Exec { port, command } => {
            let mut device = MpDevice::new(&port, 115200)?;
            let output = device.exec_command(&command)?;
            print!("{}", output);
        }
        Commands::Reset { port, hard } => {
            let mut device = MpDevice::new(&port, 115200)?;
            if hard {
                device.hard_reset()?;
            } else {
                device.soft_reset()?;
            }
        }
        Commands::Run { port, file } => {
            let mut device = MpDevice::new(&port, 115200)?;
            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Could not read {}", file.display()))?;
            let output = device.exec_command(&content)?;
            print!("{}", output);
        }
        Commands::Send {
            port,
            data,
            timeout,
        } => {
            let mut device = MpDevice::new(&port, 115200)?;
            let output = device.send_string(&data, timeout)?;
            print!("{}", output);
        }
    }

    Ok(())
}
