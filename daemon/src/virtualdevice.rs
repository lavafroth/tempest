use anyhow::{anyhow, Result};
use log::error;
use mouse_keyboard_input::VirtualDevice;
pub struct VirtualInput(VirtualDevice);

impl VirtualInput {
    pub fn new() -> Result<Self> {
        let device = VirtualDevice::default()
            .map_err(|e| anyhow!("failed to create global uinput virtual device: {e}"))?;
        Ok(Self(device))
    }
    pub fn key_chord(&mut self, keys: &[u16]) {
        for &key in keys {
            if let Err(e) = self.0.press(key) {
                error!("failed to press key {key}: {e}");
            }
        }
        for &key in keys.iter().rev() {
            if let Err(e) = self.0.release(key) {
                error!("failed to release key {key}: {e}");
            }
        }
    }
}
