import assert from "node:assert/strict";
import test from "node:test";

import { createDesktopActionDrainer, desktopAction } from "../src/desktop_actions.ts";

test("accepts only native tray actions with dashboard authority", () => {
  for (const action of ["show", "status", "capacity", "volumes", "events", "statistics", "console", "refresh", "tools", "preferences", "help", "window-hidden"]) {
    assert.equal(desktopAction(action), action);
  }
  for (const action of ["all", "management", "set-control-target", "execute-control", "get-brightness", "quit", "", null, {}]) {
    assert.equal(desktopAction(action), null);
  }
});

test("serializes drains and handles one coalesced notification without duplicate navigation", async () => {
  const responses = ["status", null, "statistics", null];
  const handled = [];
  let secondDrainRequested = false;
  const drain = createDesktopActionDrainer(async () => {
    await Promise.resolve();
    if (!secondDrainRequested) {
      secondDrainRequested = true;
      void drain();
    }
    return responses.shift() ?? null;
  }, async (action) => {
    handled.push(action);
  });

  await drain();
  assert.deepEqual(handled, ["status", "statistics"]);
});

test("invalid native values never reach the dashboard consumer", async () => {
  const responses = ["restart", "help", null];
  const handled = [];
  const drain = createDesktopActionDrainer(async () => responses.shift() ?? null, (action) => handled.push(action));

  await drain();
  assert.deepEqual(handled, ["help"]);
});
