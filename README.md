# Ghaf Control panel

Ghaf Control panel GUI application written on Rust with GTK4.
This is client which must be connected to admin service.
The address and port can be set by using args or via app menu
"Connection configuration".

**Usage**: `ctrl-panel [OPTIONS]`

**Options**:

- `--addr <ADDR>`: Admin service address (String)
- `--port <PORT>`: Admin service port (int)
- `-h, --help`: Print help