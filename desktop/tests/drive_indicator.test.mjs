import assert from "node:assert/strict";
import test from "node:test";
import { driveIndicator, fiveDBayIndicators } from "../src/drive_indicator.ts";

test("decodes only the named original SlotStatus LED values", () => {
  for (const [raw, kind] of [["1", "red"], ["2", "amber"], ["3", "green"], ["4", "protecting"], ["6", "red-flash"]]) {
    assert.equal(driveIndicator(raw, true).kind, kind);
    assert.ok(driveIndicator(raw, true).label.length);
  }
});
test("uses the original low-nibble mask without interpreting other flags", () => {
  assert.equal(driveIndicator("259", true).kind, "green");
  assert.equal(driveIndicator("4294967286", true).kind, "red-flash");
});
test("unknown selectors, malformed inputs and overflow stay neutral", () => {
  for (const raw of ["0", "5", "7", "8", "9", "15", "03", "-1", "3.0", "", "4294967299", "9".repeat(100_000), null, undefined, 3, {}]) {
    assert.equal(driveIndicator(raw, true).kind, "unknown");
  }
});
test("stale observations cannot display a reassuring or animated indicator", () => {
  for (const raw of ["1", "2", "3", "4", "6"]) assert.equal(driveIndicator(raw, false).kind, "unknown");
});

test("bay explanations distinguish capacity alerts, faults and protection activity", () => {
  assert.match(driveIndicator('1', true).label, /Capacity critical.*does not mean drive failure/);
  assert.match(driveIndicator('2', true).label, /Low space.*soon/);
  assert.match(driveIndicator('3', true).label, /No drive action indicated/);
  assert.match(driveIndicator('4', true).label, /protection.*Keep drives installed/);
  assert.match(driveIndicator('6', true).label, /failed or incompatible/);
  assert.match(driveIndicator('3', false).label, /unavailable or stale/);
});

test("inventory preserves fault bay position and never promotes missing or stale bays to ready", () => {
  const slots = [{ slot: 2, rawStatus: { mStatus: '6' } }, { slot: 0, rawStatus: { mStatus: '3' } }, { slot: 5, rawStatus: { mStatus: '1' } }];
  assert.deepEqual(fiveDBayIndicators(slots, true).map(({ slot, indicator }) => [slot, indicator.kind]), [[0, 'green'], [1, 'unknown'], [2, 'red-flash'], [3, 'unknown'], [4, 'unknown']]);
  assert.ok(fiveDBayIndicators(slots, false).every(({ indicator }) => indicator.kind === 'unknown'));
});
