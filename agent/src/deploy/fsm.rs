//! Finite State Machine for workflow deployment

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// FSM settings
#[derive(Debug, Clone)]
pub struct FsmSettings {
    /// Timeout for deployment operations
    pub deployment_timeout: Duration,

    /// Retry count for failed deployments
    pub retry_count: u32,

    /// Delay between retries
    pub retry_delay: Duration,
}

impl Default for FsmSettings {
    fn default() -> Self {
        Self {
            deployment_timeout: Duration::from_secs(60),
            retry_count: 3,
            retry_delay: Duration::from_secs(5),
        }
    }
}

/// Deployment state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentState {
    /// Initial state, not deployed
    Pending,

    /// Deployment in progress
    Deploying,

    /// Successfully deployed
    Deployed,

    /// Running (workflow is executing)
    Running,

    /// Paused
    Paused,

    /// Deployment failed
    Failed,

    /// Stopped
    Stopped,
}

/// Deployment event
#[derive(Debug, Clone)]
pub enum DeploymentEvent {
    /// Start deployment
    Deploy,

    /// Deployment completed successfully
    DeploySuccess,

    /// Deployment failed
    DeployFailed(String),

    /// Start execution
    Start,

    /// Pause execution
    Pause,

    /// Resume execution
    Resume,

    /// Stop execution
    Stop,

    /// Execution completed
    Complete,

    /// Execution error
    Error(String),

    /// Reset to pending
    Reset,
}

/// Deployment FSM
#[derive(Debug, Clone)]
pub struct DeploymentFsm {
    state: DeploymentState,
    error: Option<String>,
    retry_count: u32,
}

impl DeploymentFsm {
    /// Create a new FSM in pending state
    pub fn new() -> Self {
        Self {
            state: DeploymentState::Pending,
            error: None,
            retry_count: 0,
        }
    }

    /// Get current state
    pub fn state(&self) -> &DeploymentState {
        &self.state
    }

    /// Get error message if any
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Get retry count
    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

    /// Process an event and transition state
    pub fn process(&mut self, event: DeploymentEvent) -> Result<(), String> {
        let new_state = match (&self.state, &event) {
            // From Pending
            (DeploymentState::Pending, DeploymentEvent::Deploy) => {
                self.error = None;
                DeploymentState::Deploying
            }

            // From Deploying
            (DeploymentState::Deploying, DeploymentEvent::DeploySuccess) => {
                self.retry_count = 0;
                DeploymentState::Deployed
            }
            (DeploymentState::Deploying, DeploymentEvent::DeployFailed(err)) => {
                self.error = Some(err.clone());
                self.retry_count += 1;
                DeploymentState::Failed
            }

            // From Deployed
            (DeploymentState::Deployed, DeploymentEvent::Start) => DeploymentState::Running,
            (DeploymentState::Deployed, DeploymentEvent::Deploy) => DeploymentState::Deploying,

            // From Running
            (DeploymentState::Running, DeploymentEvent::Pause) => DeploymentState::Paused,
            (DeploymentState::Running, DeploymentEvent::Stop) => DeploymentState::Stopped,
            (DeploymentState::Running, DeploymentEvent::Complete) => DeploymentState::Deployed,
            (DeploymentState::Running, DeploymentEvent::Error(err)) => {
                self.error = Some(err.clone());
                DeploymentState::Failed
            }

            // From Paused
            (DeploymentState::Paused, DeploymentEvent::Resume) => DeploymentState::Running,
            (DeploymentState::Paused, DeploymentEvent::Stop) => DeploymentState::Stopped,

            // From Failed
            (DeploymentState::Failed, DeploymentEvent::Deploy) => {
                self.error = None;
                DeploymentState::Deploying
            }
            (DeploymentState::Failed, DeploymentEvent::Reset) => {
                self.error = None;
                self.retry_count = 0;
                DeploymentState::Pending
            }

            // From Stopped
            (DeploymentState::Stopped, DeploymentEvent::Start) => DeploymentState::Running,
            (DeploymentState::Stopped, DeploymentEvent::Deploy) => DeploymentState::Deploying,
            (DeploymentState::Stopped, DeploymentEvent::Reset) => {
                self.error = None;
                DeploymentState::Pending
            }

            // Invalid transitions
            (state, event) => {
                return Err(format!(
                    "Invalid transition: {:?} -> {:?}",
                    state, event
                ));
            }
        };

        self.state = new_state;
        Ok(())
    }

    /// Check if deployment can be retried
    pub fn can_retry(&self, max_retries: u32) -> bool {
        self.state == DeploymentState::Failed && self.retry_count < max_retries
    }
}

impl Default for DeploymentFsm {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fsm_transitions() {
        let mut fsm = DeploymentFsm::new();
        assert_eq!(fsm.state(), &DeploymentState::Pending);

        fsm.process(DeploymentEvent::Deploy).unwrap();
        assert_eq!(fsm.state(), &DeploymentState::Deploying);

        fsm.process(DeploymentEvent::DeploySuccess).unwrap();
        assert_eq!(fsm.state(), &DeploymentState::Deployed);

        fsm.process(DeploymentEvent::Start).unwrap();
        assert_eq!(fsm.state(), &DeploymentState::Running);

        fsm.process(DeploymentEvent::Pause).unwrap();
        assert_eq!(fsm.state(), &DeploymentState::Paused);

        fsm.process(DeploymentEvent::Resume).unwrap();
        assert_eq!(fsm.state(), &DeploymentState::Running);

        fsm.process(DeploymentEvent::Stop).unwrap();
        assert_eq!(fsm.state(), &DeploymentState::Stopped);
    }

    #[test]
    fn test_fsm_error_handling() {
        let mut fsm = DeploymentFsm::new();

        fsm.process(DeploymentEvent::Deploy).unwrap();
        fsm.process(DeploymentEvent::DeployFailed("test error".to_string()))
            .unwrap();

        assert_eq!(fsm.state(), &DeploymentState::Failed);
        assert_eq!(fsm.error(), Some("test error"));
        assert_eq!(fsm.retry_count(), 1);
    }
}
