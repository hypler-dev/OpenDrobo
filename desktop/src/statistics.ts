/** A real capacity reading saved by OpenDrobo. All byte values are canonical u64 decimal strings. */
export interface CapacityObservation {
  timestampMs: number;
  totalBytes: string;
  usedBytes: string;
  freeBytes: string;
  osUsedBytes: string;
}

export type StatisticsRange = "1h" | "6h" | "24h" | "7d" | "all";

/** A contiguous run of real readings. A gap is missing observation data, not an outage. */
export interface CapacitySegment {
  samples: readonly CapacityObservation[];
  startMs: number;
  endMs: number;
  spanMs: number;
}

export interface CapacityStatistics {
  /** Accepted, deduplicated, range-clipped real readings, sorted oldest first. */
  samples: readonly CapacityObservation[];
  /** Real-reading runs separated by a recording gap longer than fifteen minutes. */
  segments: readonly CapacitySegment[];
  sampleCount: number;
  rejectedCount: number;
  first: CapacityObservation | null;
  last: CapacityObservation | null;
  spanMs: number;
  largestGapMs: number;
  gapCount: number;
  usedDeltaBytes: string | null;
  freeDeltaBytes: string | null;
  osUsedDeltaBytes: string | null;
  minUsedBytes: string | null;
  maxUsedBytes: string | null;
  /** Last reading's used/total ratio, in basis points. */
  utilizationBasisPoints: number | null;
  /** Observed average daily used-byte change; never a forecast or an ETA. */
  averageDailyChangeBytes: string | null;
  capacityChanged: boolean;
}

const MAX_U64 = (1n << 64n) - 1n;
const MAX_SAMPLES = 2_048;
const FUTURE_TOLERANCE_MS = 60_000;
const GAP_MS = 15 * 60_000;
const HOUR_MS = 60 * 60_000;
const DAY_MS = 24 * HOUR_MS;

const RANGE_MS: Record<Exclude<StatisticsRange, "all">, number> = {
  "1h": HOUR_MS,
  "6h": 6 * HOUR_MS,
  "24h": DAY_MS,
  "7d": 7 * DAY_MS,
};

interface ParsedObservation {
  observation: CapacityObservation;
  totalBytes: bigint;
  usedBytes: bigint;
  freeBytes: bigint;
  osUsedBytes: bigint;
}

function parseCanonicalU64(value: unknown): bigint | null {
  if (typeof value !== "string") return null;
  if (value.length > 20) return null;
  if (!/^(?:0|[1-9][0-9]*)$/.test(value)) return null;
  const parsed = BigInt(value);
  return parsed <= MAX_U64 ? parsed : null;
}

function parseObservation(sample: unknown, futureLimitMs: number): ParsedObservation | null {
  if (typeof sample !== "object" || sample === null) return null;
  const candidate = sample as Record<string, unknown>;
  const { timestampMs, totalBytes: rawTotalBytes, usedBytes: rawUsedBytes, freeBytes: rawFreeBytes, osUsedBytes: rawOsUsedBytes } = candidate;
  if (typeof timestampMs !== "number" || !Number.isSafeInteger(timestampMs) || timestampMs <= 0 || timestampMs > futureLimitMs) return null;
  if (typeof rawTotalBytes !== "string" || typeof rawUsedBytes !== "string" || typeof rawFreeBytes !== "string" || typeof rawOsUsedBytes !== "string") return null;

  const totalBytes = parseCanonicalU64(rawTotalBytes);
  const usedBytes = parseCanonicalU64(rawUsedBytes);
  const freeBytes = parseCanonicalU64(rawFreeBytes);
  const osUsedBytes = parseCanonicalU64(rawOsUsedBytes);
  if (totalBytes === null || usedBytes === null || freeBytes === null || osUsedBytes === null) return null;
  if (totalBytes === 0n || usedBytes + freeBytes !== totalBytes) return null;

  return {
    observation: {
      timestampMs,
      totalBytes: rawTotalBytes,
      usedBytes: rawUsedBytes,
      freeBytes: rawFreeBytes,
      osUsedBytes: rawOsUsedBytes,
    },
    totalBytes,
    usedBytes,
    freeBytes,
    osUsedBytes,
  };
}

function rangeStartMs(range: StatisticsRange, nowMs: number): number {
  return range === "all" ? Number.NEGATIVE_INFINITY : nowMs - RANGE_MS[range];
}

function makeSegments(samples: readonly ParsedObservation[]): CapacitySegment[] {
  if (samples.length === 0) return [];

  const segments: CapacitySegment[] = [];
  let current: CapacityObservation[] = [samples[0].observation];
  for (let index = 1; index < samples.length; index += 1) {
    const previous = samples[index - 1].observation;
    const next = samples[index].observation;
    if (next.timestampMs - previous.timestampMs > GAP_MS) {
      segments.push({
        samples: current,
        startMs: current[0].timestampMs,
        endMs: current[current.length - 1].timestampMs,
        spanMs: current[current.length - 1].timestampMs - current[0].timestampMs,
      });
      current = [];
    }
    current.push(next);
  }
  segments.push({
    samples: current,
    startMs: current[0].timestampMs,
    endMs: current[current.length - 1].timestampMs,
    spanMs: current[current.length - 1].timestampMs - current[0].timestampMs,
  });
  return segments;
}

/**
 * Analyze saved readings only. This function creates no boundary readings,
 * interpolations, forecasts, health claims, or hardware/file/network access.
 */
export function analyzeCapacity(
  observations: readonly CapacityObservation[],
  range: StatisticsRange,
  nowMs: number,
): CapacityStatistics {
  const futureLimitMs = Math.min(Number.MAX_SAFE_INTEGER, nowMs + FUTURE_TOLERANCE_MS);
  const lastByTimestamp = new Map<number, ParsedObservation>();
  let rejectedCount = 0;

  for (const sample of observations) {
    const parsed = parseObservation(sample, futureLimitMs);
    if (parsed === null) {
      rejectedCount += 1;
      continue;
    }
    lastByTimestamp.set(parsed.observation.timestampMs, parsed);
  }

  const startMs = rangeStartMs(range, nowMs);
  const accepted = [...lastByTimestamp.values()]
    .filter((sample) => sample.observation.timestampMs >= startMs)
    .sort((left, right) => left.observation.timestampMs - right.observation.timestampMs)
    .slice(-MAX_SAMPLES);
  const samples = accepted.map((sample) => sample.observation);
  const sampleCount = samples.length;
  const first = samples[0] ?? null;
  const last = samples[sampleCount - 1] ?? null;

  let largestGapMs = 0;
  let gapCount = 0;
  for (let index = 1; index < samples.length; index += 1) {
    const gapMs = samples[index].timestampMs - samples[index - 1].timestampMs;
    if (gapMs > GAP_MS) gapCount += 1;
    if (gapMs > largestGapMs) largestGapMs = gapMs;
  }

  let minUsedBytes: bigint | null = null;
  let maxUsedBytes: bigint | null = null;
  let capacityChanged = false;
  for (const sample of accepted) {
    if (minUsedBytes === null || sample.usedBytes < minUsedBytes) minUsedBytes = sample.usedBytes;
    if (maxUsedBytes === null || sample.usedBytes > maxUsedBytes) maxUsedBytes = sample.usedBytes;
    if (first !== null && sample.totalBytes !== accepted[0].totalBytes) capacityChanged = true;
  }

  const spanMs = first === null || last === null ? 0 : last.timestampMs - first.timestampMs;
  const firstParsed = accepted[0];
  const lastParsed = accepted[sampleCount - 1];
  const hasDelta = firstParsed !== undefined && lastParsed !== undefined && sampleCount >= 2;
  const usedDelta = hasDelta ? lastParsed.usedBytes - firstParsed.usedBytes : null;
  const freeDelta = hasDelta ? lastParsed.freeBytes - firstParsed.freeBytes : null;
  const osUsedDelta = hasDelta ? lastParsed.osUsedBytes - firstParsed.osUsedBytes : null;
  const utilizationBasisPoints = lastParsed === undefined
    ? null
    : Number((lastParsed.usedBytes * 10_000n) / lastParsed.totalBytes);
  const averageDailyChangeBytes = usedDelta !== null
    && sampleCount >= 3
    && spanMs >= HOUR_MS
    && !capacityChanged
    ? ((usedDelta * BigInt(DAY_MS)) / BigInt(spanMs)).toString()
    : null;

  return {
    samples,
    segments: makeSegments(accepted),
    sampleCount,
    rejectedCount,
    first,
    last,
    spanMs,
    largestGapMs,
    gapCount,
    usedDeltaBytes: usedDelta?.toString() ?? null,
    freeDeltaBytes: freeDelta?.toString() ?? null,
    osUsedDeltaBytes: osUsedDelta?.toString() ?? null,
    minUsedBytes: minUsedBytes?.toString() ?? null,
    maxUsedBytes: maxUsedBytes?.toString() ?? null,
    utilizationBasisPoints,
    averageDailyChangeBytes,
    capacityChanged,
  };
}
