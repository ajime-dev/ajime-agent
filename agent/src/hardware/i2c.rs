//! I2C interface

use crate::errors::AgentError;

/// I2C bus wrapper
pub struct I2cBus {
    bus_number: u8,
}

impl I2cBus {
    /// Create a new I2C bus
    pub fn new(bus_number: u8) -> Result<Self, AgentError> {
        // In production, this would open the I2C bus
        Ok(Self { bus_number })
    }

    /// Get bus number
    pub fn bus_number(&self) -> u8 {
        self.bus_number
    }

    /// Scan for devices on the bus
    pub fn scan(&self) -> Result<Vec<u8>, AgentError> {
        // In production, this would scan the I2C bus
        // For now, return empty list
        Ok(Vec::new())
    }

    /// Read from a device
    pub fn read(&self, _address: u8, _register: u8, length: usize) -> Result<Vec<u8>, AgentError> {
        // In production, this would read from the I2C device
        Ok(vec![0u8; length])
    }

    /// Write to a device
    pub fn write(&self, _address: u8, _register: u8, _data: &[u8]) -> Result<(), AgentError> {
        // In production, this would write to the I2C device
        Ok(())
    }

    /// Read a single byte
    pub fn read_byte(&self, address: u8, register: u8) -> Result<u8, AgentError> {
        let data = self.read(address, register, 1)?;
        Ok(data[0])
    }

    /// Write a single byte
    pub fn write_byte(&self, address: u8, register: u8, value: u8) -> Result<(), AgentError> {
        self.write(address, register, &[value])
    }
}

/// Common I2C device addresses
pub mod addresses {
    /// BME280 temperature/humidity/pressure sensor
    pub const BME280: u8 = 0x76;

    /// MPU6050 accelerometer/gyroscope
    pub const MPU6050: u8 = 0x68;

    /// PCA9685 PWM driver
    pub const PCA9685: u8 = 0x40;

    /// SSD1306 OLED display
    pub const SSD1306: u8 = 0x3C;
}
