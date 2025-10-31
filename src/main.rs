use anyhow::{Context, Result};
use clap::Parser;
use evdev::{Device, EventType, RelativeAxisCode, uinput::VirtualDevice};
use log::{error, info};
use mouse_scroll_daemon::{AnxiousParams, AnxiousState, process_events};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the physical mouse device (e.g., /dev/input/event3)
    #[arg(short = 'D', long)]
    device: Option<PathBuf>,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.debug { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    info!("Starting anxious scroll daemon");

    // Initialize anxious parameters and state
    let anxious_params = AnxiousParams::default();
    // TODO: analyse initial jitter?
    let mut anxious_state = AnxiousState::new();

    // Find the physical mouse device
    let mut physical_device = find_mouse_device(args.device)?;
    info!(
        "Found physical mouse: {}",
        physical_device.name().unwrap_or("Unknown")
    );

    // Create virtual mouse device
    let mut virtual_device = create_virtual_mouse(&physical_device)?;
    info!("Created virtual mouse device");

    // Print virtual device paths for verification
    for path in virtual_device.enumerate_dev_nodes_blocking()? {
        let path = path?;
        info!("Virtual device available at: {}", path.display());
    }

    // Grab the physical device to get exclusive access
    physical_device
        .grab()
        .context("Failed to grab physical device")?;
    info!("Grabbed physical device for exclusive access");

    // Main event loop - pass through all events
    info!("Starting event pass-through loop...");
    run_pass_through_loop(
        &mut physical_device,
        &mut virtual_device,
        &anxious_params,
        &mut anxious_state,
    )?;

    Ok(())
}

fn find_mouse_device(device_path: Option<PathBuf>) -> Result<Device> {
    if let Some(path) = device_path {
        info!("Using specified device: {}", path.display());
        return Device::open(&path).context("Failed to open specified device");
    }

    info!("Searching for mouse devices...");
    let devices = evdev::enumerate().collect::<Vec<_>>();
    let mut best: Option<(Device, u16, std::path::PathBuf)> = None;

    for (path, device) in devices {
        let name = device.name().unwrap_or("Unknown");

        // Check if it's a mouse by looking for mouse capabilities
        let events = device.supported_events();
        if events.contains(EventType::RELATIVE) {
            if let Some(relative_axes) = device.supported_relative_axes() {
                if relative_axes.contains(RelativeAxisCode::REL_X)
                    && relative_axes.contains(RelativeAxisCode::REL_Y)
                    && relative_axes.contains(RelativeAxisCode::REL_WHEEL)
                    && relative_axes.contains(RelativeAxisCode::REL_HWHEEL)
                {
                    let input_id = device.input_id();
                    let product = input_id.product();
                    info!(
                        "Found mouse device: {} at {} (product: 0x{:04x})",
                        name,
                        path.display(),
                        product
                    );
                    match &mut best {
                        None => best = Some((device, product, path)),
                        Some((_, best_prod, _)) => {
                            if product < *best_prod {
                                best = Some((device, product, path));
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some((device, product_id, path)) = best {
        info!(
            "Selected mouse device: {} at {} (product: 0x{:04x})",
            device.name().unwrap_or("Unknown"),
            path.display(),
            product_id
        );
        return Ok(device);
    }

    anyhow::bail!("No suitable mouse device found. Please specify a device path with --device")
}

fn create_virtual_mouse(physical_device: &Device) -> Result<VirtualDevice> {
    let mut builder = VirtualDevice::builder()?.name("Anxious Scroll Daemon");

    // Add relative axes (mouse movement and scroll)
    if let Some(relative_axes) = physical_device.supported_relative_axes() {
        builder = builder.with_relative_axes(&relative_axes)?;
    }

    // Add absolute axes (if any) - skip for now as it's complex to set up properly
    // We'll focus on relative axes (mouse movement and scroll) for Phase 1

    // Add keys (mouse buttons)
    if let Some(keys) = physical_device.supported_keys() {
        builder = builder.with_keys(&keys)?;
    }

    Ok(builder.build()?)
}

fn run_pass_through_loop(
    physical_device: &mut Device,
    virtual_device: &mut VirtualDevice,
    anxious_params: &AnxiousParams,
    anxious_state: &mut AnxiousState,
) -> Result<()> {
    loop {
        match physical_device.fetch_events() {
            Ok(events) => {
                // Process events using the pure function from lib
                let event_batch = process_events(events, anxious_params, anxious_state);

                // Emit all events in the batch together
                if !event_batch.is_empty() {
                    virtual_device.emit(&event_batch)?;
                }
            }
            Err(e) => {
                error!("Error reading events: {}", e);
                // Continue the loop to keep trying
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}
