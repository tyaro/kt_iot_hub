import type {
  DriverDto,
  ImportDriverUiResultResponse,
  LaunchDriverUiResponse,
} from '$lib/ipc';

export type DriverTypeOption = {
  driverType: string;
  label: string;
  available: boolean;
  description: string;
};

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

  let attempt = 1;
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
          attempt += 1;
          continue;
        }

        setPolling(false);
        setMessage('ドライバUIが保存前に閉じられたため、取込を中止しました。必要なら再度開いてください。');
        return;
      }

      inactiveAttempts = 0;

      setMessage('ドライバUIを開いています。完了後はこの画面へ自動反映します。');
    } catch (error) {
      const message = error instanceof Error ? error.message : 'ドライバUI結果の取込に失敗しました';
      setMessage(message);
      setPolling(false);
      notify(message);
      return;
    }

    await waitFn(intervalMs);
    attempt += 1;
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
  knownDriverTypes: string[],
  driverUiAvailableByType: Record<string, boolean>,
): DriverTypeOption[] {
  const byType = new Map<string, DriverDto[]>();
  drivers.forEach((driver) => {
    if (!byType.has(driver.driver_type)) {
      byType.set(driver.driver_type, []);
    }
    byType.get(driver.driver_type)!.push(driver);
  });

  return knownDriverTypes.map((driverType) => {
    const samples = byType.get(driverType) ?? [];
    const available =
      samples.some((item) => item.registration_ui_available) ||
      (driverUiAvailableByType[driverType] ?? false);

    return {
      driverType,
      label: driverType === 'postgres' ? 'PostgreSQL 接続先' : driverType,
      available,
      description:
        driverType === 'postgres'
          ? '接続先情報、テーブル由来の Scan グループ、タグを専用UIで一括登録します。'
          : '専用UIで接続先・Scanグループ・タグを登録します。',
    };
  });
}

export async function buildDriverUiAvailabilityByType(
  knownDriverTypes: string[],
  checkDriverUiAvailable: (driverType: string, baseDir: string | null) => Promise<boolean>,
  baseDir: string | null,
): Promise<Record<string, boolean>> {
  const results = await Promise.all(
    knownDriverTypes.map(async (driverType) => {
      const available = await checkDriverUiAvailable(driverType, baseDir);
      return [driverType, available] as const;
    }),
  );

  const map: Record<string, boolean> = {};
  for (const [driverType, available] of results) {
    map[driverType] = available;
  }

  return map;
}
