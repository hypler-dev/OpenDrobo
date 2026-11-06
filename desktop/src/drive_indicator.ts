export type DriveIndicatorKind = "unknown" | "green" | "amber" | "red" | "protecting" | "red-flash";
export interface DriveIndicator { kind: DriveIndicatorKind; label: string; summary: string }

export const DRIVE_INDICATOR_MEANINGS: Record<DriveIndicatorKind, Omit<DriveIndicator, "kind">> = {
  unknown: { summary: "Status unavailable", label: "Status unavailable or stale. Refresh before assessing this bay." },
  green: { summary: "Ready", label: "Ready (steady green). No drive action indicated by this bay light; this is not a complete array-health assessment." },
  amber: { summary: "Low space", label: "Low space (steady yellow). Plan to add capacity or upgrade a drive soon." },
  red: { summary: "Add capacity now", label: "Capacity critical (solid red). Add capacity or upgrade the indicated drive; this light alone does not mean drive failure." },
  protecting: { summary: "Protecting data", label: "Data protection in progress (alternating green/yellow). Keep drives installed until protection completes." },
  "red-flash": { summary: "Drive fault", label: "Drive fault (flashing red). Review this bay for a failed or incompatible drive and replacement." },
};

/** Original Dashboard SlotStatus LED selector; this is not a complete health decoder. */
export function driveIndicator(rawStatus: unknown, fresh: boolean): DriveIndicator {
  const unknown: DriveIndicator = { kind: "unknown", ...DRIVE_INDICATOR_MEANINGS.unknown };
  if (!fresh || typeof rawStatus !== "string" || rawStatus.length > 10 || !/^(0|[1-9][0-9]*)$/.test(rawStatus)) return unknown;
  const status = BigInt(rawStatus);
  if (status > 0xffff_ffffn) return unknown;
  switch (Number(status & 0xfn)) {
    case 1: return { kind: "red", ...DRIVE_INDICATOR_MEANINGS.red };
    case 2: return { kind: "amber", ...DRIVE_INDICATOR_MEANINGS.amber };
    case 3: return { kind: "green", ...DRIVE_INDICATOR_MEANINGS.green };
    case 4: return { kind: "protecting", ...DRIVE_INDICATOR_MEANINGS.protecting };
    case 6: return { kind: "red-flash", ...DRIVE_INDICATOR_MEANINGS["red-flash"] };
    default: return unknown;
  }
}

/** Keep bay positions fixed; a missing record remains unknown, never ready. */
export function fiveDBayIndicators(slots: readonly { slot: number; rawStatus?: { mStatus?: unknown } }[], fresh: boolean): Array<{ slot: number; indicator: DriveIndicator }> {
  return Array.from({ length: 5 }, (_, slot) => ({ slot, indicator: driveIndicator(slots.find((record) => record.slot === slot)?.rawStatus?.mStatus, fresh) }));
}
