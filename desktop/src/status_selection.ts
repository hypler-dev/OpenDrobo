export interface StatusSelection {
  poolId: string | null;
  controlTargetId: string | null;
  changed: boolean;
}

/** Resolve read-only Status selection without granting device-control authority. */
export function resolveStatusSelection(poolIds: readonly string[], currentId: string | null, controlTargetId: string | null): StatusSelection {
  const poolId = currentId !== null && poolIds.includes(currentId) ? currentId : poolIds.length === 1 ? poolIds[0] : null;
  const changed = poolId !== currentId;
  return { poolId, changed, controlTargetId: !changed && controlTargetId === poolId ? controlTargetId : null };
}

/** Lengthen an opaque suffix until distinct units have distinct display labels. */
export function statusUnitLabel(id: string, poolIds: readonly string[]): string {
  let width = Math.min(6, id.length);
  while (width < id.length && poolIds.some((other) => other !== id && other.endsWith(id.slice(-width)))) width += 1;
  return `Unit ${id.slice(-width)}`;
}
