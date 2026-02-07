//! Camera interface

use crate::errors::AgentError;

/// Camera device wrapper
pub struct CameraDevice {
    device_path: String,
    width: u32,
    height: u32,
    is_open: bool,
}

impl CameraDevice {
    /// Create a new camera device
    pub fn new(device_path: &str, width: u32, height: u32) -> Self {
        Self {
            device_path: device_path.to_string(),
            width,
            height,
            is_open: false,
        }
    }

    /// Open the camera device
    pub async fn open(&mut self) -> Result<(), AgentError> {
        // In production, this would use V4L2 or similar
        // For now, just mark as open
        self.is_open = true;
        Ok(())
    }

    /// Close the camera device
    pub async fn close(&mut self) -> Result<(), AgentError> {
        self.is_open = false;
        Ok(())
    }

    /// Check if camera is open
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Capture a single frame
    pub async fn capture_frame(&self) -> Result<Vec<u8>, AgentError> {
        if !self.is_open {
            return Err(AgentError::HardwareError("Camera not open".to_string()));
        }

        // In production, this would capture from V4L2
        // For now, return a placeholder
        Ok(vec![0u8; (self.width * self.height * 3) as usize])
    }

    /// Get camera resolution
    pub fn resolution(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Get device path
    pub fn device_path(&self) -> &str {
        &self.device_path
    }
}

/// List available camera devices
pub fn list_cameras() -> Vec<String> {
    let mut cameras = Vec::new();

    // Check for V4L2 devices
    for i in 0..10 {
        let path = format!("/dev/video{}", i);
        if std::path::Path::new(&path).exists() {
            cameras.push(path);
        }
    }

    cameras
}
