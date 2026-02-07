//! Workflow models

use serde::{Deserialize, Serialize};

/// A workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Unique workflow ID
    pub id: String,

    /// Workflow name
    pub name: String,

    /// Workflow description
    pub description: Option<String>,

    /// Owner user ID
    pub owner_id: String,

    /// Workflow status
    pub status: WorkflowStatus,

    /// Graph data (nodes and edges)
    pub graph_data: GraphData,

    /// Logic hash for change detection
    pub logic_hash: Option<String>,

    /// Created timestamp
    pub created_at: String,

    /// Updated timestamp
    pub updated_at: String,
}

/// Workflow status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowStatus {
    Draft,
    Active,
    Paused,
    Archived,
}

/// Graph data containing nodes and edges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphData {
    /// Nodes in the workflow
    pub nodes: Vec<Node>,

    /// Edges connecting nodes
    pub edges: Vec<Edge>,
}

/// A node in the workflow graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Unique node ID
    pub id: String,

    /// Node type (e.g., "camera", "gpio_read", "ml_inference")
    #[serde(rename = "type")]
    pub node_type: String,

    /// Node label/name
    pub label: Option<String>,

    /// Node position in the UI
    pub position: Option<Position>,

    /// Node configuration data
    pub data: NodeData,
}

/// Node position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// Node configuration data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeData {
    /// Node-specific configuration
    #[serde(flatten)]
    pub config: serde_json::Value,

    /// Input ports
    #[serde(default)]
    pub inputs: Vec<Port>,

    /// Output ports
    #[serde(default)]
    pub outputs: Vec<Port>,
}

/// A port on a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Port {
    /// Port ID
    pub id: String,

    /// Port name
    pub name: String,

    /// Port data type
    #[serde(rename = "type")]
    pub port_type: String,
}

/// An edge connecting two nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// Unique edge ID
    pub id: String,

    /// Source node ID
    pub source: String,

    /// Source port ID
    #[serde(rename = "sourceHandle")]
    pub source_handle: Option<String>,

    /// Target node ID
    pub target: String,

    /// Target port ID
    #[serde(rename = "targetHandle")]
    pub target_handle: Option<String>,
}

/// Workflow execution state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionState {
    Idle,
    Running,
    Paused,
    Completed,
    Error,
    Cancelled,
}

/// Workflow execution context
#[derive(Debug, Clone)]
pub struct WorkflowExecution {
    /// Workflow being executed
    pub workflow: Workflow,

    /// Current execution state
    pub state: ExecutionState,

    /// Started at timestamp
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Finished at timestamp
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Error message if failed
    pub error: Option<String>,

    /// Node execution states
    pub node_states: std::collections::HashMap<String, NodeExecutionState>,
}

/// Node execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecutionState {
    /// Node ID
    pub node_id: String,

    /// Execution state
    pub state: ExecutionState,

    /// Last output values
    pub outputs: Option<serde_json::Value>,

    /// Error message if failed
    pub error: Option<String>,
}
