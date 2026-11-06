import assert from "node:assert/strict";
import test from "node:test";
import { driveIndicator } from "../src/drive_indicator.ts";

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
