import assert from "node:assert/strict";
import test from "node:test";

import { analyzeCapacity } from "../src/statistics.ts";

const HOUR = 60 * 60_000;

function reading(timestampMs, totalBytes, usedBytes, osUsedBytes = "0") {
  const total = BigInt(totalBytes);
  const used = BigInt(usedBytes);
  return {
    timestampMs,
    totalBytes: String(total),
    usedBytes: String(used),
    freeBytes: String(total - used),
    osUsedBytes,
  };
}

test("preserves u64 precision and calculates utilization without Number byte math", () => {
  const maximum = "18446744073709551615";
  const result = analyzeCapacity([
    reading(1_000, maximum, "18446744073709551613", "18446744073709551615"),
    reading(1_000 + HOUR, maximum, "18446744073709551614", "0"),
    reading(1_000 + 2 * HOUR, maximum, maximum, "1"),
  ], "all", 1_000 + 2 * HOUR);

  assert.equal(result.usedDeltaBytes, "2");
  assert.equal(result.osUsedDeltaBytes, "-18446744073709551614");
  assert.equal(result.maxUsedBytes, maximum);
  assert.equal(result.utilizationBasisPoints, 10_000);
  assert.equal(result.averageDailyChangeBytes, "24");
});

test("reports signed observed changes, including declining used capacity", () => {
  const result = analyzeCapacity([
    reading(10_000, "100", "90", "20"),
    reading(10_000 + HOUR, "100", "70", "11"),
    reading(10_000 + 2 * HOUR, "100", "40", "2"),
  ], "all", 10_000 + 2 * HOUR);

  assert.equal(result.usedDeltaBytes, "-50");
  assert.equal(result.freeDeltaBytes, "50");
  assert.equal(result.osUsedDeltaBytes, "-18");
  assert.equal(result.averageDailyChangeBytes, "-600");
});

test("refuses average rate when protected capacity changes", () => {
  const result = analyzeCapacity([
    reading(20_000, "100", "30"),
    reading(20_000 + HOUR, "200", "70"),
    reading(20_000 + 2 * HOUR, "200", "80"),
  ], "all", 20_000 + 2 * HOUR);

  assert.equal(result.capacityChanged, true);
  assert.equal(result.usedDeltaBytes, "50");
  assert.equal(result.averageDailyChangeBytes, null);
});

test("sorts out-of-order data and lets later duplicate input win", () => {
  const result = analyzeCapacity([
    reading(3_000, "100", "20"),
    reading(1_000, "100", "10"),
    reading(3_000, "100", "45"),
  ], "all", 5_000);

  assert.deepEqual(result.samples.map((sample) => sample.timestampMs), [1_000, 3_000]);
  assert.equal(result.last?.usedBytes, "45");
  assert.equal(result.rejectedCount, 0);
});

test("keeps only the newest bounded set of real readings", () => {
  const readings = Array.from({ length: 2_050 }, (_, index) => reading(index + 1, "100", "50"));
  const result = analyzeCapacity(readings, "all", 2_050);

  assert.equal(result.sampleCount, 2_048);
  assert.equal(result.first?.timestampMs, 3);
  assert.equal(result.last?.timestampMs, 2_050);
  assert.equal(readings.length, 2_050);
});

test("range clipping uses only real samples and creates no boundary point", () => {
  const now = 10 * HOUR;
  const result = analyzeCapacity([
    reading(now - 2 * HOUR, "100", "10"),
    reading(now - 30 * 60_000, "100", "30"),
  ], "1h", now);

  assert.equal(result.sampleCount, 1);
  assert.equal(result.first?.timestampMs, now - 30 * 60_000);
  assert.equal(result.usedDeltaBytes, null);
  assert.equal(result.samples[0].usedBytes, "30");
});

test("splits recording segments at gaps over fifteen minutes", () => {
  const result = analyzeCapacity([
    reading(1_000, "100", "10"),
    reading(1_000 + 15 * 60_000, "100", "20"),
    reading(1_000 + 30 * 60_000 + 1, "100", "30"),
  ], "all", 1_000 + 30 * 60_000 + 1);

  assert.equal(result.gapCount, 1);
  assert.equal(result.largestGapMs, 15 * 60_000 + 1);
  assert.deepEqual(result.segments.map((segment) => segment.samples.length), [2, 1]);
});

test("rejects invalid byte accounting, noncanonical and overflow values, and far-future readings", () => {
  const now = 1_000_000;
  const valid = reading(1, "100", "50", "18446744073709551615");
  const result = analyzeCapacity([
    valid,
    { ...reading(2, "100", "50"), totalBytes: "0100" },
    { ...reading(3, "100", "50"), usedBytes: "18446744073709551616" },
    { ...reading(4, "100", "50"), freeBytes: "49" },
    { ...reading(5, "0", "0") },
    { ...reading(6, "100", "50"), totalBytes: 100 },
    { ...reading(7, "100", "50"), osUsedBytes: { toString: () => "0" } },
    { ...reading(8, "100", "50"), totalBytes: "9".repeat(100_000) },
    null,
    reading(now + 60_001, "100", "50"),
  ], "all", now);

  assert.equal(result.rejectedCount, 9);
  assert.equal(result.sampleCount, 1);
  assert.equal(result.last?.osUsedBytes, "18446744073709551615");
});

test("returns no invented deltas or rates for empty and one-reading ranges", () => {
  const empty = analyzeCapacity([], "all", 100_000);
  assert.equal(empty.first, null);
  assert.equal(empty.last, null);
  assert.equal(empty.minUsedBytes, null);
  assert.equal(empty.utilizationBasisPoints, null);
  assert.equal(empty.averageDailyChangeBytes, null);

  const single = analyzeCapacity([reading(99_000, "100", "25")], "all", 100_000);
  assert.equal(single.usedDeltaBytes, null);
  assert.equal(single.averageDailyChangeBytes, null);
  assert.equal(single.utilizationBasisPoints, 2_500);
  assert.equal(single.minUsedBytes, "25");
  assert.equal(single.maxUsedBytes, "25");
});
