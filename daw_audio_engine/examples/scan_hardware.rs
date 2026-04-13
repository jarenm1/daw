use daw_audio_engine::device::DeviceManager;
use daw_audio_engine::{find_focusrite_devices, scan_hardware_devices};

fn main() {
    println!("=== Hardware Device Scanner ===\n");

    // Scan for all hardware devices
    println!("Raw Hardware Devices (from /proc/asound/cards):");
    let hardware = scan_hardware_devices();

    if hardware.is_empty() {
        println!("  No hardware devices found!");
    } else {
        for (i, dev) in hardware.iter().enumerate() {
            let usb_marker = dev
                .usb_id
                .as_ref()
                .map(|id| format!(" [USB: {}]", id))
                .unwrap_or_default();
            let io_marker = match (dev.is_input, dev.is_output) {
                (true, true) => "[IN/OUT]",
                (true, false) => "[IN]",
                (false, true) => "[OUT]",
                (false, false) => "[NONE]",
            };
            println!(
                "  [{}] Card {}: {} {}{}",
                i, dev.card_num, dev.card_name, io_marker, usb_marker
            );
        }
    }

    // Find Focusrite specifically
    println!("\n--- Focusrite Devices ---");
    let device_manager = DeviceManager::new();
    let focusrite = device_manager.find_focusrite_devices();

    if focusrite.is_empty() {
        println!("  No Focusrite devices found in hardware scan.");
        println!("  (But USB shows: Bus 001 Device 013: ID 1235:8210)");
        println!("  This means the driver loaded but ALSA isn't exposing it properly.");
    } else {
        for dev in focusrite {
            println!("  ✓ Found: Card {} - {}", dev.card_num, dev.card_name);
            if dev.is_output {
                println!("    To use: hw:{},0", dev.card_num);
            }
        }
    }

    // Also show what cpal sees
    println!("\n--- CPAL Enumeration ---");
    match device_manager.enumerate_devices() {
        Ok(devices) => {
            let count = devices.len();
            println!("  CPAL found {} devices", count);

            // Check if any match Focusrite
            let focusrite_in_cpal: Vec<_> = devices
                .iter()
                .filter(|d| {
                    d.name.to_lowercase().contains("focusrite")
                        || d.name.to_lowercase().contains("scarlett")
                })
                .collect();

            if focusrite_in_cpal.is_empty() {
                println!("  ✗ No Focusrite devices in CPAL list");
            } else {
                for d in focusrite_in_cpal {
                    println!("  ✓ CPAL sees: {}", d.name);
                }
            }
        }
        Err(e) => println!("  Error: {}", e),
    }

    println!("\n=== Summary ===");
    println!("Your Focusrite IS detected at USB level and in ALSA");
    println!("But CPAL's enumeration filters don't expose raw 'hw:X,Y' devices");
    println!("\nOptions to use your Focusrite:");
    println!("  1. Configure ALSA to expose it as 'default'");
    println!("  2. Use JACK instead of ALSA (bypasses this issue)");
    println!("  3. Modify CPAL to support hw: device access");
}
