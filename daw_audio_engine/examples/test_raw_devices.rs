use cpal::default_host;
use cpal::traits::{DeviceTrait, HostTrait};

fn main() {
    println!("=== Raw ALSA Device Test ===\n");

    let host = default_host();

    // List all devices cpal can see
    println!("CPAL Output Devices:");
    match host.output_devices() {
        Ok(devices) => {
            let mut count = 0;
            for (i, dev) in devices.enumerate() {
                if let Ok(name) = dev.name() {
                    println!("  [{}] {}", i, name);
                    if name.to_lowercase().contains("scarlett")
                        || name.to_lowercase().contains("focusrite")
                        || name.to_lowercase().contains("usb")
                    {
                        println!("      ^^^ FOUND FOCUSRITE!");
                    }
                    count += 1;
                }
            }
            println!("  Total: {} output devices", count);
        }
        Err(e) => println!("  Error: {}", e),
    }

    println!("\nCPAL Input Devices:");
    match host.input_devices() {
        Ok(devices) => {
            let mut count = 0;
            for (i, dev) in devices.enumerate() {
                if let Ok(name) = dev.name() {
                    println!("  [{}] {}", i, name);
                    if name.to_lowercase().contains("scarlett")
                        || name.to_lowercase().contains("focusrite")
                        || name.to_lowercase().contains("usb")
                    {
                        println!("      ^^^ FOUND FOCUSRITE!");
                    }
                    count += 1;
                }
            }
            println!("  Total: {} input devices", count);
        }
        Err(e) => println!("  Error: {}", e),
    }

    println!("\n=== ALSA Status ===");
    println!("Your Focusrite is at card 2, device 0");
    println!("Run: cat /proc/asound/cards");
    println!("Run: aplay -l | grep card\\ 2");
    println!("\nIf cpal doesn't see it, try:");
    println!("  systemctl --user restart pipewire");
    println!("  Or use JACK instead of ALSA backend");
}
