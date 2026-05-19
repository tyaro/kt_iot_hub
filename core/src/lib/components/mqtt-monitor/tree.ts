import type { MqttMonitorTopicNodeDto } from '$lib/ipc';

export type TopicTreeNode = MqttMonitorTopicNodeDto;

function compareTopicNodes(a: TopicTreeNode, b: TopicTreeNode): number {
  const aIsSys = a.label === '$SYS';
  const bIsSys = b.label === '$SYS';
  if (aIsSys !== bIsSys) {
    return aIsSys ? -1 : 1;
  }

  const left = a.label.toLowerCase();
  const right = b.label.toLowerCase();

  if (left < right) {
    return -1;
  }
  if (left > right) {
    return 1;
  }

  return a.label.localeCompare(b.label);
}

function collectNodesByPath(nodes: TopicTreeNode[], target = new Map<string, TopicTreeNode>()): Map<string, TopicTreeNode> {
  for (const node of nodes) {
    target.set(node.fullPath, node);
    collectNodesByPath(node.children, target);
  }
  return target;
}

function mergeNodeValues(current: TopicTreeNode, nextByPath: Map<string, TopicTreeNode>): TopicTreeNode {
  const nextNode = nextByPath.get(current.fullPath);

  return {
    ...current,
    hasChildren: nextNode?.hasChildren ?? current.hasChildren,
    latestMessage: nextNode?.latestMessage ?? current.latestMessage,
    children: mergeTreeValues(current.children, nextNode?.children ?? []),
  };
}

export function mergeTreeValues(current: TopicTreeNode[], next: TopicTreeNode[]): TopicTreeNode[] {
  if (current.length === 0) {
    return [...next].sort(compareTopicNodes);
  }

  const nextByPath = collectNodesByPath(next);
  const mergedCurrent = current.map((node) => mergeNodeValues(node, nextByPath));
  const currentPaths = new Set(current.map((node) => node.fullPath));
  const appendedNodes = next.filter((node) => !currentPaths.has(node.fullPath));

  return [...mergedCurrent, ...appendedNodes].sort(compareTopicNodes);
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

export function listAncestorPaths(fullPath: string | null): string[] {
  if (!fullPath) {
    return [];
  }

  const segments = fullPath.split('/').filter(Boolean);
  const ancestors: string[] = [];
  let currentPath = '';

  for (const segment of segments.slice(0, -1)) {
    currentPath = currentPath ? `${currentPath}/${segment}` : segment;
    ancestors.push(currentPath);
  }

  return ancestors;
}
