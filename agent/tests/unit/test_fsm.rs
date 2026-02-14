//! FSM unit tests

use ajigent::deploy::fsm::{DeploymentEvent, DeploymentFsm, DeploymentState};

#[test]
fn test_fsm_initial_state() {
    let fsm = DeploymentFsm::new();
    assert_eq!(fsm.state(), &DeploymentState::Pending);
    assert!(fsm.error().is_none());
    assert_eq!(fsm.retry_count(), 0);
}

#[test]
fn test_fsm_deploy_success_flow() {
    let mut fsm = DeploymentFsm::new();
    
    // Pending -> Deploying
    fsm.process(DeploymentEvent::Deploy).unwrap();
    assert_eq!(fsm.state(), &DeploymentState::Deploying);
    
    // Deploying -> Deployed
    fsm.process(DeploymentEvent::DeploySuccess).unwrap();
    assert_eq!(fsm.state(), &DeploymentState::Deployed);
    
    // Deployed -> Running
    fsm.process(DeploymentEvent::Start).unwrap();
    assert_eq!(fsm.state(), &DeploymentState::Running);
}

#[test]
fn test_fsm_deploy_failure_flow() {
    let mut fsm = DeploymentFsm::new();
    
    fsm.process(DeploymentEvent::Deploy).unwrap();
    fsm.process(DeploymentEvent::DeployFailed("test error".to_string())).unwrap();
    
    assert_eq!(fsm.state(), &DeploymentState::Failed);
    assert_eq!(fsm.error(), Some("test error"));
    assert_eq!(fsm.retry_count(), 1);
}

#[test]
fn test_fsm_retry_after_failure() {
    let mut fsm = DeploymentFsm::new();
    
    // First attempt fails
    fsm.process(DeploymentEvent::Deploy).unwrap();
    fsm.process(DeploymentEvent::DeployFailed("error 1".to_string())).unwrap();
    assert_eq!(fsm.retry_count(), 1);
    
    // Retry
    fsm.process(DeploymentEvent::Deploy).unwrap();
    fsm.process(DeploymentEvent::DeployFailed("error 2".to_string())).unwrap();
    assert_eq!(fsm.retry_count(), 2);
    
    // Check can_retry
    assert!(fsm.can_retry(3));
    assert!(!fsm.can_retry(2));
}

#[test]
fn test_fsm_pause_resume() {
    let mut fsm = DeploymentFsm::new();
    
    fsm.process(DeploymentEvent::Deploy).unwrap();
    fsm.process(DeploymentEvent::DeploySuccess).unwrap();
    fsm.process(DeploymentEvent::Start).unwrap();
    
    // Running -> Paused
    fsm.process(DeploymentEvent::Pause).unwrap();
    assert_eq!(fsm.state(), &DeploymentState::Paused);
    
    // Paused -> Running
    fsm.process(DeploymentEvent::Resume).unwrap();
    assert_eq!(fsm.state(), &DeploymentState::Running);
}

#[test]
fn test_fsm_invalid_transition() {
    let mut fsm = DeploymentFsm::new();
    
    // Cannot start from Pending
    let result = fsm.process(DeploymentEvent::Start);
    assert!(result.is_err());
}
