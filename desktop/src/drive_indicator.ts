export type DriveIndicatorKind = "unknown" | "green" | "amber" | "red" | "protecting" | "red-flash";
export interface DriveIndicator { kind: DriveIndicatorKind; label: string }

/** Original Dashboard SlotStatus LED selector; this is not a complete health decoder. */
export function driveIndicator(rawStatus: unknown, fresh: boolean): DriveIndicator {
  const unknown: DriveIndicator = { kind: "unknown", label: "Indicator unavailable or stale" };
  if (!fresh || typeof rawStatus !== "string" || rawStatus.length > 10 || !/^(0|[1-9][0-9]*)$/.test(rawStatus)) return unknown;
  const status = BigInt(rawStatus);
  if (status > 0xffff_ffffn) return unknown;
  switch (Number(status & 0xfn)) {
    case 1: return { kind: "red", label: "Red indicator reported" };
    case 2: return { kind: "amber", label: "Amber indicator reported" };
    case 3: return { kind: "green", label: "Green indicator reported" };
    case 4: return { kind: "protecting", label: "Alternating green/amber indicator reported" };
    case 6: return { kind: "red-flash", label: "Flashing red indicator reported" };
    default: return unknown;
  }
}
