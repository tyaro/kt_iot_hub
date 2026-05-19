export interface MonitorPollingOptions {
  shouldSkip: () => boolean;
  hasSelectedPath: () => boolean;
  onStatusTick: () => Promise<void>;
  onTreeTick: () => Promise<void>;
  onDetailTick: () => Promise<void>;
  onError: (message: string) => void;
  statusIntervalMs?: number;
  treeIntervalMs?: number;
  detailIntervalMs?: number;
}

export function startMonitorPolling(options: MonitorPollingOptions): () => void {
  let disposed = false;

  const safeRun = (runner: () => Promise<void>, fallbackMessage: string, guard?: () => boolean) => {
    void (async () => {
      if (disposed || options.shouldSkip() || (guard && !guard())) {
        return;
      }
      try {
        await runner();
      } catch (e) {
        if (!disposed) {
          options.onError(e instanceof Error ? e.message : fallbackMessage);
        }
      }
    })();
  };

  const statusTimerId = window.setInterval(
    () => safeRun(options.onStatusTick, 'MQTT モニタの更新に失敗しました'),
    options.statusIntervalMs ?? 2000,
  );

  const treeTimerId = window.setInterval(
    () => safeRun(options.onTreeTick, 'MQTT ツリーの更新に失敗しました'),
    options.treeIntervalMs ?? 3500,
  );

  const detailTimerId = window.setInterval(
    () => safeRun(options.onDetailTick, 'MQTT 詳細の更新に失敗しました', options.hasSelectedPath),
    options.detailIntervalMs ?? 2000,
  );

  return () => {
    disposed = true;
    window.clearInterval(statusTimerId);
    window.clearInterval(treeTimerId);
    window.clearInterval(detailTimerId);
  };
}
