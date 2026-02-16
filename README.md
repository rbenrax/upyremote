# upyremote

Herramienta CLI en Rust para interactuar con dispositivos MicroPython, inspirada en mpremote.

## Características

- **Conexión REPL interactiva**: Conecta directamente al REPL de MicroPython con soporte para historial y edición de línea
- **Transferencia de archivos**: Sube y descarga archivos usando codificación base64
- **Ejecución de comandos**: Ejecuta comandos Python de forma remota
- **Gestión del dispositivo**: Reinicio soft y hard del dispositivo
- **Modo script**: Compatible con pipes y redirección
- **Soporte multi-plataforma**: Funciona en Linux, macOS y Windows

## Instalación

### Desde código fuente

```bash
git clone <url-del-repositorio>
cd upyremote
cargo build --release
```

El binario compilado estará en `./target/release/upyremote`

### Requisitos

- Rust 1.70 o superior
- Acceso al puerto serial (usualmente requiere pertenecer al grupo `dialout` en Linux)

## Uso

### Comandos disponibles

#### `connect` - Conexión REPL interactiva

```bash
upyremote connect -p /dev/ttyACM0
```

Abre una sesión REPL interactiva con el dispositivo. Presiona `Ctrl+X` para salir.

**Atajos de teclado en modo REPL:**

| Atajo | Acción |
|-------|--------|
| `Ctrl+X` | Salir del REPL |
| `Ctrl+C` | Interrumpir programa en ejecución |
| `Ctrl+D` | EOF / Soft reset |
| `Ctrl+A` | Ir al inicio de línea |
| `Ctrl+E` | Ir al final de línea |
| `Ctrl+K` | Borrar hasta final de línea |
| `Ctrl+U` | Borrar línea completa |
| `Ctrl+W` | Borrar palabra anterior |
| `Ctrl+←` | Saltar palabra atrás |
| `Ctrl+→` | Saltar palabra adelante |
| `↑` / `↓` | Navegar historial de comandos |
| `←` / `→` | Mover cursor |
| `Home` / `End` | Ir a inicio/fin de línea |
| `Delete` | Borrar carácter bajo cursor |

#### `ls` - Listar archivos

```bash
upyremote ls -p /dev/ttyACM0 /ruta/directorio
```

Lista los archivos en el directorio especificado del dispositivo.

#### `put` - Subir archivo

```bash
upyremote put -p /dev/ttyACM0 archivo_local.py /ruta/remota/archivo.py
```

Sube un archivo local al dispositivo. Si no se especifica destino, usa el nombre del archivo local.

#### `get` - Descargar archivo

```bash
upyremote get -p /dev/ttyACM0 /ruta/remota/archivo.py archivo_local.py
```

Descarga un archivo del dispositivo. Si no se especifica destino local, usa el nombre del archivo remoto.

#### `exec` - Ejecutar comando Python

```bash
upyremote exec -p /dev/ttyACM0 "print('Hola Mundo')"
upyremote exec -p /dev/ttyACM0 "import os; print(os.listdir('/'))"
```

Ejecuta código Python en el dispositivo usando el protocolo raw REPL.

#### `run` - Ejecutar archivo Python

```bash
upyremote run -p /dev/ttyACM0 script.py
```

Lee un archivo Python local y lo ejecuta en el dispositivo.

#### `send` - Enviar cadena de texto

```bash
# Espera automáticamente al prompt (>>> o $:)
upyremote send -p /dev/ttyACM0 "print('Hola')"

# Con timeout específico (en segundos)
upyremote send -p /dev/ttyACM0 "comando" -t 5
```

Envía una cadena de texto directamente al puerto serial y muestra la respuesta. 
- Sin `-t`: Espera hasta recibir el prompt del dispositivo
- Con `-t`: Lee durante el tiempo especificado

#### `reset` - Reiniciar dispositivo

```bash
# Soft reset (Ctrl+D en MicroPython)
upyremote reset -p /dev/ttyACM0

# Hard reset (alterna señales DTR/RTS)
upyremote reset -p /dev/ttyACM0 -H
```

## Ejemplos de uso

### Conexión básica

```bash
# Conectar al REPL
upyremote connect -p /dev/ttyACM0

# En el REPL, puedes usar:
# >>> print("Hola")
# >>> import os
# >>> os.listdir('/')
```

### Gestión de archivos

```bash
# Subir un script
upyremote put -p /dev/ttyACM0 main.py

# Descargar un archivo de log
upyremote get -p /dev/ttyACM0 /log.txt backup_log.txt

# Ver archivos en el directorio raíz
upyremote ls -p /dev/ttyACM0 /
```

### Ejecución de comandos

```bash
# Ejecutar código simple
upyremote exec -p /dev/ttyACM0 "print(2+2)"

# Ver información del sistema
upyremote exec -p /dev/ttyACM0 "import sys; print(sys.version)"

# Listar archivos
upyremote exec -p /dev/ttyACM0 "import os; print(os.listdir('/'))"
```

### Uso en scripts

```bash
# Enviar múltiples comandos
echo -e "x = 100\nprint(x)" | upyremote connect -p /dev/ttyACM0

# Automatizar tareas
upyremote send -p /dev/ttyACM0 "import machine; machine.freq()" -t 2
```

## Opciones globales

Cada comando acepta las siguientes opciones:

- `-p, --port <PORT>`: Puerto serial (default: `/dev/ttyUSB0`)
  - Linux: `/dev/ttyUSB0`, `/dev/ttyACM0`
  - macOS: `/dev/cu.usbserial*`, `/dev/cu.usbmodem*`
  - Windows: `COM3`, `COM4`, etc.

## Solución de problemas

### Permiso denegado al acceder al puerto

En Linux, añade tu usuario al grupo `dialout`:

```bash
sudo usermod -a -G dialout $USER
# Cerrar sesión y volver a iniciar
```

### Dispositivo no encontrado

Verifica que el dispositivo esté conectado:

```bash
# Linux
ls -la /dev/ttyACM* /dev/ttyUSB*

# macOS
ls -la /dev/cu.*
```

### Puerto ocupado

Si recibes "Device or resource busy", verifica que no haya otro proceso usando el puerto:

```bash
lsof /dev/ttyACM0
# o
fuser /dev/ttyACM0
```

## Desarrollo

### Compilar en modo debug

```bash
cargo build
```

### Compilar en modo release (optimizado)

```bash
cargo build --release
```

### Ejecutar tests

```bash
cargo test
```

## Arquitectura

El proyecto utiliza:
- **clap**: Parser de argumentos de línea de comandos
- **serialport**: Comunicación serial multiplataforma
- **crossterm**: Manejo de terminal en modo raw para el REPL interactivo
- **anyhow**: Manejo de errores

## Licencia

MIT License - Ver LICENSE para más detalles.

## Contribuciones

Las contribuciones son bienvenidas. Por favor, abre un issue o pull request.

## Agradecimientos

Inspirado en [mpremote](https://docs.micropython.org/en/latest/reference/mpremote.html), la herramienta oficial de MicroPython.
# upyremote
