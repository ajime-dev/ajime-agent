//! MQTT topic definitions

/// MQTT topic patterns
pub struct Topics;

impl Topics {
    /// Device command topic
    pub fn device_command(device_id: &str) -> String {
        format!("ajime/device/{}/command", device_id)
    }

    /// Device status topic
    pub fn device_status(device_id: &str) -> String {
        format!("ajime/device/{}/status", device_id)
    }

    /// Device telemetry topic
    pub fn device_telemetry(device_id: &str) -> String {
        format!("ajime/device/{}/telemetry", device_id)
    }

    /// Workflow control topic
    pub fn workflow_control(workflow_id: &str) -> String {
        format!("ajime/workflow/{}/control", workflow_id)
    }

    /// Workflow status topic
    pub fn workflow_status(workflow_id: &str) -> String {
        format!("ajime/workflow/{}/status", workflow_id)
    }

    /// Parse a topic to extract the device ID
    pub fn parse_device_id(topic: &str) -> Option<String> {
        let parts: Vec<&str> = topic.split('/').collect();
        if parts.len() >= 3 && parts[0] == "ajime" && parts[1] == "device" {
            Some(parts[2].to_string())
        } else {
            None
        }
    }

    /// Parse a topic to extract the workflow ID
    pub fn parse_workflow_id(topic: &str) -> Option<String> {
        let parts: Vec<&str> = topic.split('/').collect();
        if parts.len() >= 3 && parts[0] == "ajime" && parts[1] == "workflow" {
            Some(parts[2].to_string())
        } else {
            None
        }
    }

    /// Check if topic is a command topic
    pub fn is_command_topic(topic: &str) -> bool {
        topic.ends_with("/command")
    }

    /// Check if topic is a control topic
    pub fn is_control_topic(topic: &str) -> bool {
        topic.ends_with("/control")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_generation() {
        assert_eq!(
            Topics::device_command("device-123"),
            "ajime/device/device-123/command"
        );
        assert_eq!(
            Topics::workflow_control("workflow-456"),
            "ajime/workflow/workflow-456/control"
        );
    }

    #[test]
    fn test_topic_parsing() {
        assert_eq!(
            Topics::parse_device_id("ajime/device/device-123/command"),
            Some("device-123".to_string())
        );
        assert_eq!(
            Topics::parse_workflow_id("ajime/workflow/workflow-456/control"),
            Some("workflow-456".to_string())
        );
    }
}
