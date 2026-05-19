export type MetricItem = {
  icon?: string;
  label: string;
  value: string;
  tone?: 'normal' | 'warn' | 'danger';
};

export type ScanCycleHealthSummary = {
  observedGroupCount: number;
  delayedGroupCount: number;
  avgDeltaRatio: number | null;
  worstGroupLabel: string | null;
  worstDeltaRatio: number | null;
};
