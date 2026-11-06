export const desktopActionNames = [
  "show",
  "status",
  "capacity",
  "volumes",
  "events",
  "statistics",
  "console",
  "refresh",
  "tools",
  "preferences",
  "help",
  "window-hidden"
] as const;

export type DesktopAction = typeof desktopActionNames[number];

const desktopActionSet = new Set<string>(desktopActionNames);

export function desktopAction(value: unknown): DesktopAction | null {
  return typeof value === "string" && desktopActionSet.has(value) ? value as DesktopAction : null;
}

export function createDesktopActionDrainer(
  takeAction: () => Promise<unknown>,
  consumeAction: (action: DesktopAction) => void | Promise<void>
): () => Promise<void> {
  let draining = false;
  let drainRequested = false;

  return async () => {
    if (draining) {
      drainRequested = true;
      return;
    }
    draining = true;
    try {
      do {
        drainRequested = false;
        for (let reads = 0; reads < 64; reads += 1) {
          const value = await takeAction();
          if (value === null) break;
          const action = desktopAction(value);
          if (action === null) continue;
          await consumeAction(action);
        }
      } while (drainRequested);
    } finally {
      draining = false;
    }
  };
}
