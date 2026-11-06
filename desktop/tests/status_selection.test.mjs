import assert from "node:assert/strict";
import test from "node:test";
import { resolveStatusSelection, statusUnitLabel } from "../src/status_selection.ts";

test("automatically opens the sole unit without selecting a control target", () => {
  assert.deepEqual(resolveStatusSelection(["pool-a"], null, null), { poolId: "pool-a", controlTargetId: null, changed: true });
});

test("multiple units require an explicit Status choice, independent of ordering", () => {
  for (const ids of [["pool-a", "pool-b"], ["pool-b", "pool-a"]]) {
    assert.deepEqual(resolveStatusSelection(ids, null, "pool-b"), { poolId: null, controlTargetId: null, changed: false });
  }
});

test("retains an existing choice and only its previously selected matching control target", () => {
  assert.deepEqual(resolveStatusSelection(["pool-b", "pool-a"], "pool-a", "pool-a"), { poolId: "pool-a", controlTargetId: "pool-a", changed: false });
  assert.equal(resolveStatusSelection(["pool-a"], "pool-a", "pool-b").controlTargetId, null);
});

test("read-only selection never creates a Tools target", () => {
  assert.equal(resolveStatusSelection(["pool-a", "pool-b"], "pool-b", null).controlTargetId, null);
  assert.equal(resolveStatusSelection(["pool-a"], "pool-a", null).controlTargetId, null);
});

test("disappearing units clear authority even when the remaining unit opens automatically", () => {
  assert.deepEqual(resolveStatusSelection(["pool-b"], "pool-a", "pool-a"), { poolId: "pool-b", controlTargetId: null, changed: true });
  assert.deepEqual(resolveStatusSelection(["pool-b", "pool-c"], "pool-a", "pool-a"), { poolId: null, controlTargetId: null, changed: true });
  assert.equal(resolveStatusSelection(["pool-b"], "pool-a", "pool-b").controlTargetId, null);
});

test("empty discovery clears the displayed unit and control binding", () => {
  assert.deepEqual(resolveStatusSelection([], "pool-a", "pool-a"), { poolId: null, controlTargetId: null, changed: true });
});

test("identical models with colliding opaque suffixes still get distinct stable labels", () => {
  const ids = ["device-a-abcdef", "device-b-abcdef", "unrelated-123456"];
  const labels = ids.map((id) => statusUnitLabel(id, ids));
  assert.equal(new Set(labels).size, ids.length);
  assert.equal(labels[2], "Unit 123456");
  for (const [index, id] of ids.entries()) assert.equal(statusUnitLabel(id, [...ids].reverse()), labels[index]);
});
