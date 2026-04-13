use crate::error::{AudioError, AudioResult};
use log::info;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct HardwareDevice {
    pub card_num: u32,
    pub card_name: String,
    pub usb_id: Option<String>,
    pub is_input: bool,
    pub is_output: bool,
}

/// Scan /proc/asound/cards and aplay/arecord to find raw hardware devices
pub fn scan_hardware_devices() -> Vec<HardwareDevice> {
    let mut devices = Vec::new();

    // Read /proc/asound/cards
    if let Ok(cards) = std::fs::read_to_string("/proc/asound/cards") {
        for line in cards.lines() {
            // Format: " 2 [USB            ]: USB-Audio - Scarlett 2i2 USB"
            if line.starts_with(" ") && line.contains("[") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(card_num) = parts[0].parse::<u32>() {
                        let card_name = line.split("]:").nth(1).unwrap_or(line).trim().to_string();

                        // Check if this is USB audio
                        let usb_id = find_usb_id_for_card(card_num);

                        // Check capabilities via aplay/arecord
                        let (has_output, has_input) = check_device_capabilities(card_num);

                        devices.push(HardwareDevice {
                            card_num,
                            card_name: card_name.clone(),
                            usb_id,
                            is_input: has_input,
                            is_output: has_output,
                        });

                        info!(
                            "Found hardware card {}: {} (out:{}, in:{})",
                            card_num, card_name, has_output, has_input
                        );
                    }
                }
            }
        }
    }

    devices
}

fn find_usb_id_for_card(card_num: u32) -> Option<String> {
    let path = format!("/proc/asound/card{}/usbid", card_num);
    std::fs::read_to_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
}

fn check_device_capabilities(card_num: u32) -> (bool, bool) {
    let mut has_output = false;
    let mut has_input = false;

    // Check for playback devices
    if let Ok(output) = Command::new("aplay").args(&["-l"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains(&format!("card {}", card_num)) {
                has_output = true;
                break;
            }
        }
    }

    // Check for capture devices
    if let Ok(output) = Command::new("arecord").args(&["-l"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains(&format!("card {}", card_num)) {
                has_input = true;
                break;
            }
        }
    }

    (has_output, has_input)
}

/// Create ALSA device name for cpal
pub fn get_alsa_device_name(card_num: u32, device_num: u32, is_input: bool) -> String {
    if is_input {
        format!("hw:{},{}", card_num, device_num)
    } else {
        format!("hw:{},{}", card_num, device_num)
    }
}

/// Find Focusrite devices specifically
pub fn find_focusrite_devices() -> Vec<HardwareDevice> {
    scan_hardware_devices()
        .into_iter()
        .filter(|d| {
            d.card_name.to_lowercase().contains("focusrite")
                || d.card_name.to_lowercase().contains("scarlett")
                || d.usb_id
                    .as_ref()
                    .map(|id| id.starts_with("1235:"))
                    .unwrap_or(false)
        })
        .collect()
}
