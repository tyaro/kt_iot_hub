import type { MqttMonitorMessageDto } from '$lib/ipc';

export type TopicTreeNode = {
  id: string;
  label: string;
  fullPath: string;
  latestMessage: MqttMonitorMessageDto | null;
  children: TopicTreeNode[];
};

type MutableNode = {
  id: string;
  label: string;
  fullPath: string;
  latestMessage: MqttMonitorMessageDto | null;
  children: Map<string, MutableNode>;
};

function createNode(label: string, fullPath: string): MutableNode {
  return {
    id: fullPath || label,
    label,
    fullPath,
    latestMessage: null,
    children: new Map(),
  };
}

function freezeNode(node: MutableNode): TopicTreeNode {
  return {
    id: node.id,
    label: node.label,
    fullPath: node.fullPath,
    latestMessage: node.latestMessage,
    children: Array.from(node.children.values())
      .map(freezeNode)
      .sort((a, b) => a.label.localeCompare(b.label)),
  };
}

export function buildTopicTree(messages: MqttMonitorMessageDto[]): TopicTreeNode[] {
  const roots = new Map<string, MutableNode>();

  for (const message of messages) {
    const segments = message.topic.split('/').filter(Boolean);
    if (segments.length === 0) {
      continue;
    }

    let currentMap = roots;
    let currentNode: MutableNode | null = null;
    let currentPath = '';

    for (const segment of segments) {
      currentPath = currentPath ? `${currentPath}/${segment}` : segment;
      let nextNode = currentMap.get(segment);
      if (!nextNode) {
        nextNode = createNode(segment, currentPath);
        currentMap.set(segment, nextNode);
      }
      nextNode.latestMessage = message;
      currentNode = nextNode;
      currentMap = nextNode.children;
    }

    if (currentNode) {
      currentNode.latestMessage = message;
    }
  }

  return Array.from(roots.values())
    .map(freezeNode)
    .sort((a, b) => a.label.localeCompare(b.label));
}

export function findNodeByPath(nodes: TopicTreeNode[], fullPath: string | null): TopicTreeNode | null {
  if (!fullPath) {
    return null;
  }

  const stack = [...nodes];
  while (stack.length > 0) {
    const node = stack.pop();
    if (!node) {
      continue;
    }
    if (node.fullPath === fullPath) {
      return node;
    }
    stack.push(...node.children);
  }

  return null;
}
