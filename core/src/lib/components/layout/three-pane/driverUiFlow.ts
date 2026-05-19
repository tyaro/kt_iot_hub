import type {
  DiscoveredDriverPackageDto,
  DriverDto,
  InvalidDriverPackageDto,
  ImportDriverUiResultResponse,
  LaunchDriverUiResponse,
} from '$lib/ipc';

export type DriverTypeOption = {
  driverType: string;
  label: string;
  available: boolean;
  description: string;
  statusMessage?: string;
};

function toDriverUiErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof Error) {
    return error.message || fallback;
  }

  if (typeof error === 'string' && error.trim().length > 0) {
    return error;
  }

  if (error && typeof error === 'object') {
    const raw = error as Record<string, unknown>;
    const code = typeof raw.code === 'string' ? raw.code : null;
    const message = typeof raw.error === 'string'
      ? raw.error
      : typeof raw.message === 'string'
        ? raw.message
        : null;

    if (message && code) {
      return `${code}: ${message}`;
    }
    if (message) {
      return message;
    }

    try {
      return JSON.stringify(raw);
    } catch {
      return fallback;
    }
  }

  return fallback;
}

function driverTypeLabel(driverType: string): string {
  switch (driverType) {
    case 'postgres':
      return 'PostgreSQL 接続先';
    case 'joywatcher':
      return 'JoyWatcher 接続先';
    default:
      return driverType;
  }
}

function driverTypeDescription(driverType: string): string {
  switch (driverType) {
    case 'postgres':
      return '接続先情報、テーブル由来の Scan グループ、タグを専用UIで一括登録します。';
    case 'joywatcher':
      return 'TagSel2 でタグを取り込み、接続先・Scan グループ・タグを専用UIでまとめて登録します。';
    default:
      return '専用UIで接続先・Scanグループ・タグを登録します。';
  }
}

export type MonitorDriverUiImportDeps = {
  result: LaunchDriverUiResponse;
  token: number;
  isTokenValid: (token: number) => boolean;
  setMessage: (message: string) => void;
  setPolling: (polling: boolean) => void;
  checkReady: (
    outputJsonPath: string,
    sessionId: string,
  ) => Promise<{ ready: boolean; process_active: boolean }>;
  importResult: (req: {
    session_id: string;
    driver_id?: string | null;
    output_json_path: string;
  }) => Promise<ImportDriverUiResultResponse>;
  onImported: (imported: ImportDriverUiResultResponse) => Promise<void>;
  waitFn: (ms: number) => Promise<void>;
  notify: (message: string) => void;
  intervalMs?: number;
  inactiveGraceAttempts?: number;
};

export async function monitorDriverUiImport({
  result,
  token,
  isTokenValid,
  setMessage,
  setPolling,
  checkReady,
  importResult,
  onImported,
  waitFn,
  notify,
  intervalMs = 1000,
  inactiveGraceAttempts = 5,
}: MonitorDriverUiImportDeps): Promise<void> {
  setPolling(true);

  let inactiveAttempts = 0;
  while (isTokenValid(token)) {
    if (!isTokenValid(token)) {
      return;
    }

    try {
      const check = await checkReady(result.output_json_path, result.session_id);
      if (check.ready) {
        const imported = await importResult({
          session_id: result.session_id,
          driver_id: result.driver_id,
          output_json_path: result.output_json_path,
        });
        await onImported(imported);
        setPolling(false);
        return;
      }

      if (!check.process_active) {
        inactiveAttempts += 1;
        if (inactiveAttempts <= inactiveGraceAttempts) {
          setMessage('ドライバUI終了後の保存結果を確認しています...');
          await waitFn(intervalMs);
          continue;
        }

        setPolling(false);
        setMessage('ドライバUIが保存前に閉じられたため、取込を中止しました。必要なら再度開いてください。');
        return;
      }

      inactiveAttempts = 0;

      setMessage('ドライバUIを開いています。完了後はこの画面へ自動反映します。');
    } catch (error) {
      const message = toDriverUiErrorMessage(error, 'ドライバUI結果の取込に失敗しました');
      setMessage(message);
      setPolling(false);
      notify(message);
      return;
    }

    await waitFn(intervalMs);
  }
}

export function canUseDriverUiForDriver(
  driverId: string,
  drivers: DriverDto[],
  driverUiAvailableByType: Record<string, boolean>,
): boolean {
  const driver = drivers.find((item) => item.id === driverId);
  if (!driver) {
    return false;
  }

  return (
    driver.registration_ui_available ||
    (driverUiAvailableByType[driver.driver_type] ?? false)
  );
}

export function buildDriverTypeOptions(
  drivers: DriverDto[],
  discoveredPackages: DiscoveredDriverPackageDto[],
  invalidPackages: InvalidDriverPackageDto[],
  driverUiAvailableByType: Record<string, boolean>,
): DriverTypeOption[] {
  const byType = new Map<string, DriverDto[]>();
  drivers.forEach((driver) => {
    if (!byType.has(driver.driver_type)) {
      byType.set(driver.driver_type, []);
    }
    byType.get(driver.driver_type)!.push(driver);
  });

  const invalidByType = new Map<string, InvalidDriverPackageDto[]>();
  invalidPackages.forEach((item) => {
    if (!item.driver_type_hint) {
      return;
    }

    const items = invalidByType.get(item.driver_type_hint) ?? [];
    items.push(item);
    invalidByType.set(item.driver_type_hint, items);
  });

  const orderedTypes: string[] = [];
  const pushType = (driverType: string) => {
    if (driverType.trim().length === 0 || orderedTypes.includes(driverType)) {
      return;
    }
    orderedTypes.push(driverType);
  };

  discoveredPackages.forEach((pkg) => pushType(pkg.driver_type));
  invalidPackages.forEach((pkg) => {
    if (pkg.driver_type_hint) {
      pushType(pkg.driver_type_hint);
    }
  });
  drivers.forEach((driver) => pushType(driver.driver_type));

  return orderedTypes.map((driverType) => {
    const samples = byType.get(driverType) ?? [];
    const discovered = discoveredPackages.find((pkg) => pkg.driver_type === driverType) ?? null;
    const invalids = invalidByType.get(driverType) ?? [];
    const available =
      Boolean(discovered) ||
      samples.some((item) => item.registration_ui_available) ||
      (driverUiAvailableByType[driverType] ?? false);

    const baseDescription = discovered?.display_name
      ? `${discovered.display_name} / ${driverTypeDescription(driverType)}`
      : driverTypeDescription(driverType);

    const manifestDetails = discovered
      ? [
          discovered.version ? `version ${discovered.version}` : null,
          discovered.vendor ? `vendor ${discovered.vendor}` : null,
        ]
          .filter((value): value is string => Boolean(value))
          .join(' / ')
      : '';

    const invalidSummary = invalids[0]?.status_message ?? null;

    return {
      driverType,
      label: discovered?.display_name ?? driverTypeLabel(driverType),
      available,
      description: [baseDescription, manifestDetails].filter(Boolean).join(' / '),
      statusMessage: invalidSummary ?? undefined,
    };
  });
}

export function buildDriverUiAvailabilityByType(
  discoveredPackages: DiscoveredDriverPackageDto[],
): Record<string, boolean> {
  const map: Record<string, boolean> = {};
  for (const pkg of discoveredPackages) {
    map[pkg.driver_type] = true;
  }

  return map;
}
