//! GPIO interface

use crate::errors::AgentError;

/// GPIO pin mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinMode {
    Input,
    Output,
}

/// GPIO pin state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinState {
    Low,
    High,
}

impl From<bool> for PinState {
    fn from(value: bool) -> Self {
        if value {
            PinState::High
        } else {
            PinState::Low
        }
    }
}

impl From<PinState> for bool {
    fn from(state: PinState) -> Self {
        matches!(state, PinState::High)
    }
}

/// GPIO pin wrapper
pub struct GpioPin {
    pin: u8,
    mode: PinMode,
}

impl GpioPin {
    /// Create a new GPIO pin
    pub fn new(pin: u8, mode: PinMode) -> Result<Self, AgentError> {
        // In production, this would initialize the GPIO pin
        // using rppal or sysfs
        Ok(Self { pin, mode })
    }

    /// Get pin number
    pub fn pin(&self) -> u8 {
        self.pin
    }

    /// Get pin mode
    pub fn mode(&self) -> PinMode {
        self.mode
    }

    /// Read pin state (for input pins)
    pub fn read(&self) -> Result<PinState, AgentError> {
        if self.mode != PinMode::Input {
            return Err(AgentError::HardwareError(
                "Cannot read from output pin".to_string(),
            ));
        }

        // In production, this would read from the actual GPIO
        Ok(PinState::Low)
    }

    /// Write pin state (for output pins)
    pub fn write(&mut self, state: PinState) -> Result<(), AgentError> {
        if self.mode != PinMode::Output {
            return Err(AgentError::HardwareError(
                "Cannot write to input pin".to_string(),
            ));
        }

        // In production, this would write to the actual GPIO
        Ok(())
    }

    /// Set pin high
    pub fn set_high(&mut self) -> Result<(), AgentError> {
        self.write(PinState::High)
    }

    /// Set pin low
    pub fn set_low(&mut self) -> Result<(), AgentError> {
        self.write(PinState::Low)
    }

    /// Toggle pin state
    pub fn toggle(&mut self) -> Result<(), AgentError> {
        let current = self.read()?;
        match current {
            PinState::Low => self.set_high(),
            PinState::High => self.set_low(),
        }
    }
}

/// GPIO controller for managing multiple pins
pub struct GpioController {
    pins: std::collections::HashMap<u8, GpioPin>,
}

impl GpioController {
    /// Create a new GPIO controller
    pub fn new() -> Self {
        Self {
            pins: std::collections::HashMap::new(),
        }
    }

    /// Setup a pin
    pub fn setup_pin(&mut self, pin: u8, mode: PinMode) -> Result<(), AgentError> {
        let gpio_pin = GpioPin::new(pin, mode)?;
        self.pins.insert(pin, gpio_pin);
        Ok(())
    }

    /// Get a pin reference
    pub fn get_pin(&self, pin: u8) -> Option<&GpioPin> {
        self.pins.get(&pin)
    }

    /// Get a mutable pin reference
    pub fn get_pin_mut(&mut self, pin: u8) -> Option<&mut GpioPin> {
        self.pins.get_mut(&pin)
    }

    /// Release a pin
    pub fn release_pin(&mut self, pin: u8) {
        self.pins.remove(&pin);
    }

    /// Release all pins
    pub fn release_all(&mut self) {
        self.pins.clear();
    }
}

impl Default for GpioController {
    fn default() -> Self {
        Self::new()
    }
}
