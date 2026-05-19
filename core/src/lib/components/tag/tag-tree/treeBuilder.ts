import type { DriverDto, ScanGroupDto, TagDto } from '$lib/ipc';

export interface GroupedScanGroupNode {
  scanGroup: ScanGroupDto;
  tags: TagDto[];
}

export interface GroupedDriverNode {
  driver: DriverDto;
  scanGroups: GroupedScanGroupNode[];
}

export function buildGroupedTree(
  drivers: DriverDto[],
  scanGroups: ScanGroupDto[],
  tags: TagDto[],
): GroupedDriverNode[] {
  const tagsByScanGroup = new Map<string, TagDto[]>();

  tags.forEach((tag) => {
    if (!tagsByScanGroup.has(tag.scan_group_id)) {
      tagsByScanGroup.set(tag.scan_group_id, []);
    }
    tagsByScanGroup.get(tag.scan_group_id)!.push(tag);
  });

  const scanGroupsByDriver = new Map<string, ScanGroupDto[]>();
  scanGroups.forEach((scanGroup) => {
    if (!scanGroupsByDriver.has(scanGroup.driver_id)) {
      scanGroupsByDriver.set(scanGroup.driver_id, []);
    }
    scanGroupsByDriver.get(scanGroup.driver_id)!.push(scanGroup);
  });

  return [...drivers]
    .sort((a, b) => a.id.localeCompare(b.id))
    .map((driver) => {
      const nodes = [...(scanGroupsByDriver.get(driver.id) ?? [])]
        .sort((a, b) => a.id.localeCompare(b.id))
        .map((scanGroup) => ({
          scanGroup,
          tags: [...(tagsByScanGroup.get(scanGroup.id) ?? [])].sort((a, b) => a.name.localeCompare(b.name)),
        }));

      return { driver, scanGroups: nodes };
    });
}
