# UOF Systemd Service Templates

This directory contains systemd service unit files for deploying UOF components.

## Services

### uof-agent.service
Node daemon that manages eBPF probes and forwards events to the control plane.

### uof-control-plane.service
State management API server that coordinates agents and plugins.

## Deployment

1. Copy service files to systemd directory:
   ```bash
   sudo cp uof-agent.service /etc/systemd/system/
   sudo cp uof-control-plane.service /etc/systemd/system/
   ```

2. Create configuration directory:
   ```bash
   sudo mkdir -p /etc/uof
   ```

3. Create configuration files (see examples below)

4. Reload systemd and enable services:
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable uof-agent
   sudo systemctl enable uof-control-plane
   ```

5. Start services:
   ```bash
   sudo systemctl start uof-control-plane
   sudo systemctl start uof-agent
   ```

## Configuration

### Agent (`/etc/uof/agent.toml`)
```toml
control_plane_url = "http://localhost:8080"
agent_id = "node-1"
```

### Control Plane (`/etc/uof/control-plane.toml`)
```toml
port = 8080
database_url = "postgresql://uof:uof@localhost:5432/uof"
```

## Troubleshooting

View logs:
```bash
journalctl -u uof-agent -f
journalctl -u uof-control-plane -f
```

Restart services:
```bash
sudo systemctl restart uof-agent
sudo systemctl restart uof-control-plane
```