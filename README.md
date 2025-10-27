# Anxious Scroll Daemon

A custom userspace mouse scroll wheel daemon that intercepts and modifies scroll events from your physical mouse using the Linux evdev/uinput subsystem. Transform your mouse's scroll behavior with smooth, dynamic sensitivity that adapts to your scrolling speed.

## Overview

This project addresses the issue where the mouse firmware's "smart" scroll wheel behavior doesn't align with user preferences. Instead of modifying kernel drivers or system libraries, this daemon operates in userspace as a man-in-the-middle between your physical mouse and the input system.


## 🏗️ Architecture

```
Physical Mouse → evdev → Our Daemon → uinput → Virtual Mouse → libinput → Xorg → Applications
```

## 🧮 Scroll Transformation Algorithm

The basic idea is based on quake-live acceleration, where the faster you scroll, the faster the "acceleration" multiplier is. But instead of a bunch of ramp up and ramp down functions, I've used a smooth signmoid curve instead.

![Logistic Function Visualization](images/logit.png)

The daemon uses a logistic function for smooth, natural acceleration:

```
f(velocity) = max_sensitivity / (1 + C * e^(-ramp_rate * velocity))
```

Now at zero veloctity, we would like `base_sensitivity`

```
C = (max_sensitivity / base_sensitivity) - 1
```

Where:
- `base_sensitivity`: Starting sensitivity (default: 1.0)
- `max_sensitivity`: Maximum sensitivity (default: 15.0) 
- `ramp_rate`: How quickly to accelerate (default: 0.3)

This creates a smooth curve that starts slow for precision and ramps up for speed.

## 📦 Installation as System Service

### Quick Installation

To install the daemon as a system service that starts automatically on boot:

```bash
# Install and optionally start the service
sudo ./install.sh

# Uninstall the service
sudo ./uninstall.sh
```

### Manual Installation

If you prefer to install manually:

```bash
# Build the release binary
cargo build --release

# Copy binary to system location
sudo cp target/release/anxious-scroll-daemon /usr/local/bin/
sudo chmod +x /usr/local/bin/anxious-scroll-daemon

# Copy service file
sudo cp anxious-scroll-daemon.service /etc/systemd/system/

# Enable and start the service
sudo systemctl daemon-reload
sudo systemctl enable anxious-scroll-daemon
sudo systemctl start anxious-scroll-daemon
```

### Service Management

```bash
# Check service status
sudo systemctl status anxious-scroll-daemon

# Start the service
sudo systemctl start anxious-scroll-daemon

# Stop the service
sudo systemctl stop anxious-scroll-daemon

# Restart the service
sudo systemctl restart anxious-scroll-daemon

# View live logs
sudo journalctl -u anxious-scroll-daemon -f

# View recent logs
sudo journalctl -u anxious-scroll-daemon --since "1 hour ago"
```

### Troubleshooting

If the service fails to start or doesn't detect your mouse:

1. **Check service logs**: `sudo journalctl -u anxious-scroll-daemon -f`
2. **Find your mouse device**: `ls -l /dev/input/by-id/`
3. **Test device manually**: `sudo evtest /dev/input/eventX`
4. **Specify device manually**: Edit `/etc/systemd/system/anxious-scroll-daemon.service` and add `--device /dev/input/eventX` to the ExecStart line

### Finding Your Mouse Device

```bash
# List all input devices
ls -l /dev/input/by-id/

# Test with evtest to see events
sudo evtest /dev/input/event3
```
