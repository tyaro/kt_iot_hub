use crate::app_state::MqttMonitorMessageState;
use crate::commands::dto::{
    MqttMonitorMessageDto, MqttMonitorTopicNodeDto,
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
struct MutableTopicNode {
    label: String,
    full_path: String,
    latest_message: Option<MqttMonitorMessageDto>,
    children: HashMap<String, MutableTopicNode>,
}

impl MutableTopicNode {
    fn new(label: String, full_path: String) -> Self {
        Self {
            label,
            full_path,
            latest_message: None,
            children: HashMap::new(),
        }
    }
}

pub(super) fn build_visible_topic_tree(
    topics: &HashMap<String, MqttMonitorMessageState>,
    expanded_paths: &HashSet<String>,
    publisher_topic_root: Option<String>,
    include_sys: bool,
    topic_filter: &str,
    include_all: bool,
) -> Vec<MqttMonitorTopicNodeDto> {
    let mut roots = HashMap::<String, MutableTopicNode>::new();

    if let Some(root) = publisher_topic_root
        .or_else(|| first_fixed_topic_segment(topic_filter).map(|segment| segment.to_string()))
    {
        roots
            .entry(root.clone())
            .or_insert_with(|| MutableTopicNode::new(root.clone(), root));
    }

    if include_sys {
        roots
            .entry("$SYS".to_string())
            .or_insert_with(|| MutableTopicNode::new("$SYS".to_string(), "$SYS".to_string()));
    }

    for message in topics.values() {
        let segments = message
            .topic
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();
        if segments.is_empty() {
            continue;
        }

        let message_dto: MqttMonitorMessageDto = message.clone().into();
        let mut current_map = &mut roots;
        let mut current_path = String::new();

        for segment in segments {
            if !current_path.is_empty() {
                current_path.push('/');
            }
            current_path.push_str(segment);

            let node = current_map
                .entry(segment.to_string())
                .or_insert_with(|| MutableTopicNode::new(segment.to_string(), current_path.clone()));
            node.latest_message = Some(message_dto.clone());
            current_map = &mut node.children;
        }
    }

    let mut nodes = roots
        .into_values()
        .map(|node| freeze_visible_node(node, expanded_paths, include_all))
        .collect::<Vec<_>>();
    sort_topic_nodes(&mut nodes);
    nodes
}

fn freeze_visible_node(
    node: MutableTopicNode,
    expanded_paths: &HashSet<String>,
    include_all: bool,
) -> MqttMonitorTopicNodeDto {
    let has_children = !node.children.is_empty();
    let mut children = if has_children && (include_all || expanded_paths.contains(&node.full_path)) {
        node.children
            .into_values()
            .map(|child| freeze_visible_node(child, expanded_paths, include_all))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    sort_topic_nodes(&mut children);

    MqttMonitorTopicNodeDto {
        id: node.full_path.clone(),
        label: node.label,
        full_path: node.full_path,
        has_children,
        latest_message: node.latest_message,
        children,
    }
}

fn sort_topic_nodes(nodes: &mut [MqttMonitorTopicNodeDto]) {
    nodes.sort_by(|a, b| {
        let a_is_sys = a.label == "$SYS";
        let b_is_sys = b.label == "$SYS";

        a_is_sys
            .cmp(&b_is_sys)
            .reverse()
            .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
            .then_with(|| a.label.cmp(&b.label))
    });
}

pub(super) fn find_latest_message_for_path(
    topics: &HashMap<String, MqttMonitorMessageState>,
    full_path: &str,
) -> Option<MqttMonitorMessageDto> {
    topics
        .values()
        .filter(|message| {
            message.topic == full_path || message.topic.starts_with(&format!("{}/", full_path))
        })
        .max_by(|a, b| a.timestamp.cmp(&b.timestamp))
        .cloned()
        .map(Into::into)
}

pub(super) fn first_fixed_topic_segment(topic_filter: &str) -> Option<&str> {
    topic_filter
        .split('/')
        .map(str::trim)
        .find(|segment| !segment.is_empty() && *segment != "#" && *segment != "+")
}

#[cfg(test)]
mod tests {
    use super::{build_visible_topic_tree, first_fixed_topic_segment};
    use crate::app_state::MqttMonitorMessageState;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn fixed_topic_segment_skips_wildcards() {
        assert_eq!(first_fixed_topic_segment("+/factory/#"), Some("factory"));
        assert_eq!(first_fixed_topic_segment("#"), None);
    }

    #[test]
    fn build_tree_contains_root_and_child() {
        let mut topics = HashMap::new();
        topics.insert(
            "factory/line1/temp".to_string(),
            MqttMonitorMessageState {
                timestamp: "2026-05-18T00:00:00Z".to_string(),
                topic: "factory/line1/temp".to_string(),
                payload: "42".to_string(),
                qos: 0,
                retain: false,
            },
        );

        let expanded = HashSet::from(["factory".to_string(), "factory/line1".to_string()]);
        let nodes = build_visible_topic_tree(
            &topics,
            &expanded,
            Some("factory".to_string()),
            false,
            "factory/#",
            false,
        );

        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].label, "factory");
        assert!(!nodes[0].children.is_empty());
    }
}
