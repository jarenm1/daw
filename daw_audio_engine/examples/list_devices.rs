use daw_audio_engine::device::DeviceManager;
use daw_audio_engine::{AudioDeviceInfo, AudioEngine};
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== DAW Audio Engine Demo ===\n");

    let mut engine = AudioEngine::new()?;
    let device_manager = DeviceManager::new();

    // Check for hardware devices (Focusrite, etc.)
    let hardware_devices = device_manager.enumerate_hardware_devices();
    let focusrite_devices: Vec<_> = hardware_devices
        .iter()
        .filter(|d| {
            d.card_name.to_lowercase().contains("focusrite")
                || d.card_name.to_lowercase().contains("scarlett")
        })
        .collect();

    // Get cpal devices
    let cpal_devices = engine.list_devices()?;

    println!("Hardware Devices Found:");
    println!("{:-<60}", "");

    let mut device_index = 0;
    let mut hw_index_map: Vec<usize> = Vec::new();

    if !focusrite_devices.is_empty() {
        println!("\n*** FOCUSRITE DEVICES (Direct Hardware) ***");
        for dev in focusrite_devices.iter() {
            let io = match (dev.is_input, dev.is_output) {
                (true, true) => "IN/OUT",
                (true, false) => "INPUT",
                (false, true) => "OUTPUT",
                _ => "",
            };
            println!(
                "  [{}] {} - {} [USB: {}]",
                device_index,
                dev.card_name,
                io,
                dev.usb_id.as_deref().unwrap_or("unknown")
            );
            println!("      Device name: hw:{},0", dev.card_num);
            hw_index_map.push(dev.card_num as usize);
            device_index += 1;
        }
    }

    let cpal_focusrite: Vec<_> = cpal_devices
        .iter()
        .filter(|d| {
            d.name.to_lowercase().contains("focusrite")
                || d.name.to_lowercase().contains("scarlett")
        })
        .collect();

    if !cpal_focusrite.is_empty() {
        println!("\n--- Focusrite in CPAL List ---");
        for dev in cpal_focusrite.iter() {
            println!("  [{}] {}", device_index, dev);
            device_index += 1;
        }
    }

    if !focusrite_devices.is_empty() && cpal_focusrite.is_empty() {
        println!("\n⚠ Your Focusrite is detected at USB/ALSA level but NOT in CPAL's list");
        println!("   This is a known issue - CPAL uses ALSA plugins, not raw hardware");
    }

    // Regular devices
    let other_devices: Vec<_> = cpal_devices
        .iter()
        .filter(|d| {
            !d.name.to_lowercase().contains("focusrite")
                && !d.name.to_lowercase().contains("scarlett")
        })
        .collect();

    let cpal_start_index = device_index;
    if !other_devices.is_empty() {
        println!("\n--- CPAL Audio Devices ---");
        for (i, device) in other_devices.iter().enumerate() {
            println!("  [{}] {}", device_index + i, device);
        }
        device_index += other_devices.len();
    }

    if focusrite_devices.is_empty() && cpal_focusrite.is_empty() && other_devices.is_empty() {
        println!("  No audio devices found!");
        return Ok(());
    }

    print!("\nSelect device (0-{}): ", device_index - 1);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let selection: usize = input.trim().parse()?;

    let config = daw_audio_engine::EngineConfig {
        sample_rate: 48000,
        buffer_size: 128, // Lower for better latency
        channels: 2,
        input_enabled: false,
        output_enabled: true,
    };

    // Check if hardware device selected
    if selection < focusrite_devices.len() {
        let dev = focusrite_devices[selection];
        println!(
            "\n⚠ Cannot directly use hardware device '{}' yet",
            dev.card_name
        );
        println!(
            "   CPAL doesn't support raw ALSA device names like 'hw:{},0'",
            dev.card_num
        );
        println!("\n   Workarounds:");
        println!("   1. Use 'default' device (routes through PipeWire/ALSA)");
        println!("   2. Set Focusrite as system default in audio settings");
        println!("   3. Use JACK backend (requires implementation)");
        println!("\nFalling back to default device...");
        engine.start()?;
    } else if selection >= cpal_start_index {
        // CPAL device
        let cpal_index = selection - cpal_start_index;
        let selected_device = &other_devices[cpal_index];

        println!("\nStarting audio engine with: {}", selected_device.name);
        engine.start_with_device(&selected_device.name, config)?;
    } else {
        // Focusrite in CPAL list (unlikely)
        let cpal_index = selection - focusrite_devices.len();
        let selected_device = &cpal_focusrite[cpal_index];
        println!("\nStarting audio engine with: {}", selected_device.name);
        engine.start_with_device(&selected_device.name, config)?;
    }

    let latency = engine.current_latency_ms().unwrap_or(0.0);
    println!("Audio engine running!");
    println!("  Latency: {:.1}ms", latency);
    println!("  Target: 5-10ms");

    if latency <= 10.0 {
        println!("  Status: ✓ Low latency achieved!");
    } else {
        println!("  Status: ⚠ Higher latency");
    }

    println!("\nPress Enter to stop...");
    io::stdin().read_line(&mut String::new())?;

    engine.stop()?;
    println!("\nAudio engine stopped.");

    Ok(())
}
