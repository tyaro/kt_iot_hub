export type MetricTone = 'normal' | 'warn' | 'danger';

export function formatBytes(value?: number | null): string {
  if (value == null || !Number.isFinite(value)) {
    return '-';
  }
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let size = value;
  let index = 0;
  while (size >= 1024 && index < units.length - 1) {
    size /= 1024;
    index += 1;
  }
  return `${size.toFixed(index === 0 ? 0 : 1)} ${units[index]}`;
}

export function formatPercent(value?: number | null): string {
  if (value == null || !Number.isFinite(value)) {
    return '-';
  }
  return `${value.toFixed(1)}%`;
}

export function formatByteRate(value?: number | null): string {
  if (value == null || !Number.isFinite(value)) {
    return '-';
  }
  if (value < 1024) {
    return `${value.toFixed(1)} B`;
  }
  return formatBytes(value);
}

export function cpuLevel(value?: number | null): MetricTone {
  if (value == null || !Number.isFinite(value)) {
    return 'normal';
  }
  if (value >= 90) {
    return 'danger';
  }
  if (value >= 70) {
    return 'warn';
  }
  return 'normal';
}

export function ioLevel(value?: number | null): MetricTone {
  if (value == null || !Number.isFinite(value)) {
    return 'normal';
  }
  if (value >= 10 * 1024 * 1024) {
    return 'danger';
  }
  if (value >= 1024 * 1024) {
    return 'warn';
  }
  return 'normal';
}
