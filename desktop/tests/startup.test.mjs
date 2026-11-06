import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

const source = readFileSync(new URL("../public/startup.js", import.meta.url), "utf8");

function startup() {
  let now = 0;
  let nextTimer = 0;
  const timers = new Map();
  const listeners = new Map();
  const elements = new Map();
  const document = {
    activeElement: null,
    getElementById: (id) => elements.get(id),
    querySelector: () => elements.get("main"),
  };
  for (const id of ["opendrobo-startup", "opendrobo-startup-status", "opendrobo-startup-dismiss", "opendrobo-startup-error", "main"]) {
    const classes = new Set();
    elements.set(id, {
      hidden: id === "opendrobo-startup-error",
      classList: { add: (name) => classes.add(name), contains: (name) => classes.has(name) },
      focus() { document.activeElement = this; },
      contains: (element) => element === elements.get("opendrobo-startup-dismiss"),
      addEventListener: (name, callback) => listeners.set(`${id}:${name}`, new Set([callback])),
    });
  }
  const window = {
    setTimeout: (callback, delay) => { const id = ++nextTimer; timers.set(id, { at: now + delay, callback }); return id; },
    requestAnimationFrame: (callback) => window.setTimeout(callback, 16),
    clearTimeout: (id) => timers.delete(id),
    addEventListener: (name, callback) => { if (!listeners.has(name)) listeners.set(name, new Set()); listeners.get(name).add(callback); },
    removeEventListener: (name, callback) => listeners.get(name)?.delete(callback),
  };
  vm.runInNewContext(source, { document, window, Date: { now: () => now } });
  return {
    api: window.OpenDroboStartup,
    error: elements.get("opendrobo-startup-error"),
    dismissed: () => elements.get("opendrobo-startup").classList.contains("is-dismissed"),
    emit: (name, event = {}) => { for (const callback of listeners.get(name) ?? []) callback(event); },
    advance(to) {
      for (;;) {
        const due = [...timers].filter(([, timer]) => timer.at <= to).sort((a, b) => a[1].at - b[1].at)[0];
        if (!due) break;
        timers.delete(due[0]); now = due[1].at; due[1].callback();
      }
      now = to;
    },
  };
}

test("ready shell dismisses startup after a bounded visible interval without hardware", () => {
  const view = startup();
  view.api.dismissWhenReady();
  view.advance(15);
  assert.equal(view.dismissed(), false);
  view.advance(731);
  assert.equal(view.dismissed(), false);
  view.advance(732);
  assert.equal(view.dismissed(), true);
  view.advance(5000);
  assert.equal(view.error.hidden, true);
});

test("manual skip cannot suppress failure fallback when the main module never loads", () => {
  const view = startup();
  view.emit("keydown", { key: "Escape" });
  assert.equal(view.dismissed(), true);
  view.advance(3000);
  assert.equal(view.error.hidden, false);
});

test("failed startup exposes persistent recovery screen and dismisses overlay", () => {
  const view = startup();
  view.emit("error");
  assert.equal(view.error.hidden, false);
  view.advance(1200);
  assert.equal(view.dismissed(), true);
});

test("later application errors do not revive the startup failure screen", () => {
  const view = startup();
  view.api.dismissWhenReady();
  view.advance(732);
  view.emit("error");
  view.emit("unhandledrejection");
  view.advance(5000);
  assert.equal(view.error.hidden, true);
});

test("a ready shell clears an earlier startup fallback", () => {
  const view = startup();
  view.emit("error");
  view.api.dismissWhenReady();
  view.advance(5000);
  assert.equal(view.error.hidden, true);
  assert.equal(view.dismissed(), true);
});
