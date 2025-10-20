# Anxious Scroll Daemon

A custom userspace mouse scroll wheel daemon that intercepts and modifies scroll events from your physical mouse using the Linux evdev/uinput subsystem.

## Overview

This project addresses the issue where the mouse firmware's "smart" scroll wheel behavior doesn't align with user preferences. Instead of modifying kernel drivers or system libraries, this daemon operates in userspace as a man-in-the-middle between your physical mouse and the input system.

## How It Works

```
Physical Mouse → evdev → Our Daemon → uinput → Virtual Mouse → libinput → Xorg → Applications
```

## Status

### Building!

