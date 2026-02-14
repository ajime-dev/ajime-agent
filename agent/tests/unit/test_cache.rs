//! Cache unit tests

use ajigent::cache::workflow::WorkflowCache;
use ajigent::models::workflow::{GraphData, Workflow, WorkflowStatus};

fn create_test_workflow(id: &str, name: &str) -> Workflow {
    Workflow {
        id: id.to_string(),
        name: name.to_string(),
        description: None,
        owner_id: "test-owner".to_string(),
        status: WorkflowStatus::Active,
        graph_data: GraphData {
            nodes: vec![],
            edges: vec![],
        },
        logic_hash: Some("test-hash".to_string()),
        created_at: "2025-01-01T00:00:00Z".to_string(),
        updated_at: "2025-01-01T00:00:00Z".to_string(),
    }
}

#[test]
fn test_workflow_cache_insert_and_get() {
    let cache = WorkflowCache::new(10);
    let workflow = create_test_workflow("wf-1", "Test Workflow");
    
    cache.insert(workflow.clone(), "digest-1".to_string());
    
    let entry = cache.get("wf-1");
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().workflow.name, "Test Workflow");
}

#[test]
fn test_workflow_cache_eviction() {
    let cache = WorkflowCache::new(2);
    
    cache.insert(create_test_workflow("wf-1", "Workflow 1"), "d1".to_string());
    cache.insert(create_test_workflow("wf-2", "Workflow 2"), "d2".to_string());
    cache.insert(create_test_workflow("wf-3", "Workflow 3"), "d3".to_string());
    
    // Cache should have evicted the oldest entry
    assert_eq!(cache.len(), 2);
}

#[test]
fn test_workflow_cache_remove() {
    let cache = WorkflowCache::new(10);
    let workflow = create_test_workflow("wf-1", "Test Workflow");
    
    cache.insert(workflow, "digest-1".to_string());
    assert!(cache.get("wf-1").is_some());
    
    cache.remove("wf-1");
    assert!(cache.get("wf-1").is_none());
}
