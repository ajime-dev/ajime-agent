//! Workflow executor

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::deploy::fsm::{DeploymentEvent, DeploymentFsm, DeploymentState};
use crate::deploy::node_runner::{NodeRunner, NodeRunnerFactory};
use crate::errors::AgentError;
use crate::models::workflow::{ExecutionState, Workflow, WorkflowExecution};

/// Workflow executor
pub struct WorkflowExecutor {
    workflow: Workflow,
    fsm: RwLock<DeploymentFsm>,
    node_runners: RwLock<HashMap<String, Arc<dyn NodeRunner>>>,
    execution: RwLock<Option<WorkflowExecution>>,
}

impl WorkflowExecutor {
    /// Create a new workflow executor
    pub fn new(workflow: Workflow) -> Self {
        Self {
            workflow,
            fsm: RwLock::new(DeploymentFsm::new()),
            node_runners: RwLock::new(HashMap::new()),
            execution: RwLock::new(None),
        }
    }

    /// Get the workflow
    pub fn workflow(&self) -> &Workflow {
        &self.workflow
    }

    /// Get the current deployment state
    pub async fn state(&self) -> DeploymentState {
        self.fsm.read().await.state().clone()
    }

    /// Deploy the workflow
    pub async fn deploy(&self) -> Result<(), AgentError> {
        info!("Deploying workflow: {}", self.workflow.name);

        // Transition to deploying
        {
            let mut fsm = self.fsm.write().await;
            fsm.process(DeploymentEvent::Deploy)
                .map_err(|e| AgentError::DeployError(e))?;
        }

        // Create node runners
        match self.create_node_runners().await {
            Ok(_) => {
                let mut fsm = self.fsm.write().await;
                fsm.process(DeploymentEvent::DeploySuccess)
                    .map_err(|e| AgentError::DeployError(e))?;
                info!("Workflow deployed successfully: {}", self.workflow.name);
                Ok(())
            }
            Err(e) => {
                let mut fsm = self.fsm.write().await;
                fsm.process(DeploymentEvent::DeployFailed(e.to_string()))
                    .map_err(|e| AgentError::DeployError(e))?;
                Err(e)
            }
        }
    }

    async fn create_node_runners(&self) -> Result<(), AgentError> {
        let mut runners = self.node_runners.write().await;
        runners.clear();

        for node in &self.workflow.graph_data.nodes {
            let runner = NodeRunnerFactory::create(node)?;
            runners.insert(node.id.clone(), runner);
            debug!("Created runner for node: {} ({})", node.id, node.node_type);
        }

        Ok(())
    }

    /// Start workflow execution
    pub async fn start(&self) -> Result<(), AgentError> {
        info!("Starting workflow: {}", self.workflow.name);

        // Transition to running
        {
            let mut fsm = self.fsm.write().await;
            fsm.process(DeploymentEvent::Start)
                .map_err(|e| AgentError::DeployError(e))?;
        }

        // Create execution context
        {
            let mut execution = self.execution.write().await;
            *execution = Some(WorkflowExecution {
                workflow: self.workflow.clone(),
                state: ExecutionState::Running,
                started_at: Some(chrono::Utc::now()),
                finished_at: None,
                error: None,
                node_states: HashMap::new(),
            });
        }

        // Start execution loop
        self.run_execution_loop().await
    }

    async fn run_execution_loop(&self) -> Result<(), AgentError> {
        // This is a simplified execution loop
        // In production, this would handle message passing between nodes
        
        let runners = self.node_runners.read().await;
        
        for (node_id, runner) in runners.iter() {
            debug!("Executing node: {}", node_id);
            
            // Execute node with empty inputs (simplified)
            match runner.execute(HashMap::new()).await {
                Ok(outputs) => {
                    debug!("Node {} completed with {} outputs", node_id, outputs.len());
                }
                Err(e) => {
                    error!("Node {} failed: {}", node_id, e);
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    /// Stop workflow execution
    pub async fn stop(&self) -> Result<(), AgentError> {
        info!("Stopping workflow: {}", self.workflow.name);

        {
            let mut fsm = self.fsm.write().await;
            fsm.process(DeploymentEvent::Stop)
                .map_err(|e| AgentError::DeployError(e))?;
        }

        // Update execution state
        {
            let mut execution = self.execution.write().await;
            if let Some(ref mut exec) = *execution {
                exec.state = ExecutionState::Cancelled;
                exec.finished_at = Some(chrono::Utc::now());
            }
        }

        Ok(())
    }

    /// Pause workflow execution
    pub async fn pause(&self) -> Result<(), AgentError> {
        info!("Pausing workflow: {}", self.workflow.name);

        {
            let mut fsm = self.fsm.write().await;
            fsm.process(DeploymentEvent::Pause)
                .map_err(|e| AgentError::DeployError(e))?;
        }

        // Update execution state
        {
            let mut execution = self.execution.write().await;
            if let Some(ref mut exec) = *execution {
                exec.state = ExecutionState::Paused;
            }
        }

        Ok(())
    }

    /// Resume workflow execution
    pub async fn resume(&self) -> Result<(), AgentError> {
        info!("Resuming workflow: {}", self.workflow.name);

        {
            let mut fsm = self.fsm.write().await;
            fsm.process(DeploymentEvent::Resume)
                .map_err(|e| AgentError::DeployError(e))?;
        }

        // Update execution state
        {
            let mut execution = self.execution.write().await;
            if let Some(ref mut exec) = *execution {
                exec.state = ExecutionState::Running;
            }
        }

        Ok(())
    }

    /// Get execution status
    pub async fn get_execution(&self) -> Option<WorkflowExecution> {
        self.execution.read().await.clone()
    }
}
