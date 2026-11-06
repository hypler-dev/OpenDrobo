import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { analyzeCapacity, type CapacityObservation, type CapacityStatistics, type StatisticsRange } from "./statistics";
import { resolveStatusSelection, statusUnitLabel } from "./status_selection";
import { driveIndicator } from "./drive_indicator";
import "./styles.css";

type Section = "all" | "status" | "capacity" | "volumes" | "events" | "statistics" | "console" | "sharing" | "mcp" | "management" | "tools" | "settings" | "preferences" | "help";
type CapacityState = "available" | "unavailable" | "error";
type UpdateState = "none" | "available" | "current" | "error";
type ConsoleLevel = "info" | "warning" | "error";
type ConsoleLevelFilter = ConsoleLevel | "all";
type EventSeverity = "info" | "warning" | "error";
type EventSeverityFilter = EventSeverity | "all";
type ControlAction = "identify" | "brightness" | "get-brightness" | "restart" | "shutdown" | "standby";
type ObservationState = "active" | "inactive" | "unknown" | "stale" | "read-error";

interface Device {
  id: string;
  name: string;
  connection: string;
  driverReady: boolean;
}

interface InventoryResponse {
  platform: string;
  devices: Device[];
  message: string;
}

interface CapacityResponse {
  deviceId: string;
  state: CapacityState;
  protectedTotalBytes: string | null;
  protectedUsedBytes: string | null;
  protectedFreeBytes: string | null;
  observedAtMs: number | null;
  message: string;
}

interface UpdateResponse {
  state: UpdateState;
  message: string;
  releaseUrl: string | null;
}

interface AppInfoResponse {
  version: string;
  platform: string;
  filesystemModuleBuilt: boolean;
  mcpCommandPath: string | null;
  message: string;
}

interface StorageVolume {
  id: string;
  name: string;
  filesystem: string;
  mountPath: string | null;
  totalBytes: string | null;
  usedBytes: string | null;
  availableBytes: string | null;
  deviceId: string | null;
  provider: "drobo" | "host";
  accounting: "os-logical";
  readOnly: boolean;
}

interface StorageStatusResponse {
  platform: string;
  volumes: StorageVolume[];
  message: string;
}

interface ConsoleRecord {
  id: string;
  timestampMs: number;
  level: ConsoleLevel;
  source: string;
  operation: string;
  message: string;
}

interface ConsoleResponse {
  records: ConsoleRecord[];
  persistence: "disk" | "memory";
  message: string;
}

interface SharingService {
  id: string;
  name: string;
  state: "running" | "stopped" | "unavailable" | "unknown";
  supported: boolean;
  message: string;
}

interface SharingExport {
  id: string;
  name: string;
  path: string;
  protocol: string;
  readOnly: boolean | null;
  guestAllowed: boolean | null;
}

interface SharingStatusResponse {
  platform: string;
  services: SharingService[];
  exports: SharingExport[];
  exportsAvailable: boolean;
  configuration: {
    permissions: "os-native";
    settingsActionAvailable: boolean;
    message: string;
  };
  message: string;
}

interface PoolVolume {
  lun: number;
  logicalTotalBytes: string;
  osUsedBytes: string;
}

interface PoolSlot {
  slot: number;
  kind: "ssd" | "drive";
  physicalCapacityBytes: string;
  ssdLifeRemainingPercent: number | null;
  cacheUsedBytes: null;
  manufacturerModel?: string | null;
  firmwareRevision?: string | null;
  driveState: "active" | "inactive" | "unknown";
  rawStatus: {
    mStatus: string;
    mDiskState: string;
    mDiskType: string;
  };
}

interface PoolStatus {
  id: string;
  brand: string;
  model: string;
  firmwareVersion?: string | null;
  firmwareReleaseDate?: string | null;
  connectionType?: number;
  unitState: "active" | "inactive" | "unknown";
  deviceStatus: {
    mStatus: string;
    mStatusEx: string;
    mDiskPackStatus: string;
  };
  lunCount: number;
  protectedTotalBytes: string;
  protectedUsedBytes: string;
  protectedFreeBytes: string;
  osUsedBytes: string;
  volumes: PoolVolume[];
  slots: PoolSlot[];
}

interface PoolStatusResponse {
  state: "available";
  source: "original-service";
  observedAtMs: number;
  unitValidation: "service-reported-bytes";
  reportingScope: "shared-pool";
  filesystemReportingEnabled: false;
  sourceTrust?: "unverified-loopback";
  pools: PoolStatus[];
  message: string;
}

interface CapacityHistorySample {
  timestampMs: number;
  totalBytes: string;
  usedBytes: string;
  freeBytes: string;
  osUsedBytes: string;
}

interface CapacityHistoryPool {
  id: string;
  model: string;
  samples: CapacityHistorySample[];
}

interface CapacityHistoryResponse {
  state: "available" | "unavailable";
  source: "opendrobo-observations";
  persistence: "disk" | "memory";
  pools: CapacityHistoryPool[];
  message: string;
}

interface DeviceEvent {
  id: string;
  timestampMs: number;
  severity: EventSeverity;
  source: string;
  unitId: string | null;
  message: string;
  rawHealthState: string | null;
  kind?: string;
  slot?: number | null;
}

interface ObservedEventSlot {
  slot: number;
  state: ObservationState;
}

interface ObservedEventUnit {
  id: string;
  state: ObservationState;
  lastObservedAtMs: number | null;
  slots: ObservedEventSlot[];
}

interface DeviceEventsResponse {
  events: DeviceEvent[];
  observedSinceMs: number | null;
  historyScope: "observed-since-observation";
  state: ObservationState;
  lastObservedAtMs: number | null;
  units: ObservedEventUnit[];
  message: string;
}

interface DeviceControl {
  action: ControlAction;
  label: string;
  enabled: boolean;
  reason: string;
  requiresConfirmation: boolean;
  minimum: number | null;
  maximum: number | null;
}

interface ControlTarget {
  id: string;
  brand: string;
  model: string;
  controls: DeviceControl[];
}

interface ControlStatusResponse {
  state: "available" | "unavailable";
  targets: ControlTarget[];
  message: string;
}

interface ControlResult {
  state: "accepted" | "rejected" | "unknown";
  action: ControlAction;
  message: string;
  value?: number | null;
}

const sections: Array<{ id: Section; label: string; icon: string; artwork?: string; activeArtwork?: string }> = [
  { id: "all", label: "All Drobos", icon: "▦", artwork: "DroboNav-all_devices.png", activeArtwork: "DroboNav-all_devicesA.png" },
  { id: "status", label: "Status", icon: "♥", artwork: "DroboNav-status.png", activeArtwork: "DroboNav-statusA.png" },
  { id: "capacity", label: "Capacity", icon: "▣", artwork: "DroboNav-capacity.png", activeArtwork: "DroboNav-capacityA.png" },
  { id: "volumes", label: "Volumes", icon: "▤", artwork: "DroboNav-shares_volumes.png", activeArtwork: "DroboNav-shares_volumes_active.png" },
  { id: "events", label: "Events", icon: "◉" },
  { id: "statistics", label: "Statistics", icon: "◒" },
  { id: "sharing", label: "Sharing", icon: "⇄" },
  { id: "console", label: "Console", icon: "›_" },
  { id: "mcp", label: "MCP", icon: "◇" },
  { id: "management", label: "Reference", icon: "◌" },
  { id: "tools", label: "Tools", icon: "⌘", artwork: "DroboNav-tools.png", activeArtwork: "DroboNav-toolsA.png" },
  { id: "settings", label: "Drobo Settings", icon: "⚙", artwork: "DroboNav-settings.png", activeArtwork: "DroboNav-settingsA.png" },
  { id: "preferences", label: "Dashboard Prefs", icon: "≡", artwork: "DroboNav-prefs.png", activeArtwork: "DroboNav-prefsA.png" },
  { id: "help", label: "Help", icon: "?", artwork: "DroboNav-help.png", activeArtwork: "DroboNav-helpA.png" }
];

const artwork = ["DroboMiniArtwork.png", "Drobo5DArtwork.png", "DroboProArtwork.png", "DroboB800Artwork.png"];
const capacityMaxAgeMs = 30_000;
const app = requiredElement("app");

let selectedSection: Section = "all";
let selectedDeviceId: string | null = null;
let selectedPoolId: string | null = null;
let selectedStatusView: "System Information" | "Drive Information" = "System Information";
let selectedBaySlot: number | null = null;
let capacityMode: "capacity" | "usage" = "capacity";
let inventory: InventoryResponse | null = null;
let inventoryError: string | null = null;
let inventoryLoading = false;
let capacity: CapacityResponse | null = null;
let capacityRequestedAtMs: number | null = null;
let capacityLoading = false;
let updateResult: UpdateResponse | null = null;
let updateLoading = false;
let appInfo: AppInfoResponse | null = null;
let poolStatus: PoolStatusResponse | null = null;
let poolError: string | null = null;
let poolStaleError: string | null = null;
let poolLoading = false;
let poolRequestSequence = 0;
let poolExpiryTimer: number | null = null;
let poolEventSubscribed = false;
let statisticsRange: StatisticsRange = "all";
let statisticsMetric: "used" | "free" | "os" = "used";
let statisticsPoolId: string | null = null;
let capacityHistory: CapacityHistoryResponse | null = null;
let capacityHistoryError: string | null = null;
let capacityHistoryLoading = false;
let capacityHistoryRequestSequence = 0;
let storageStatus: StorageStatusResponse | null = null;
let storageError: string | null = null;
let storageLoading = false;
let storageRequestSequence = 0;
let sharingStatus: SharingStatusResponse | null = null;
let sharingError: string | null = null;
let sharingLoading = false;
let sharingRequestSequence = 0;
let sharingSettingsStatus: string | null = null;
let consoleResponse: ConsoleResponse | null = null;
let consoleError: string | null = null;
let consoleLoading = false;
let consolePaused = false;
let consoleSearch = "";
let consoleLevelFilter: ConsoleLevelFilter = "all";
let consoleRequestSequence = 0;
let consolePollTimer: number | null = null;
let eventsResponse: DeviceEventsResponse | null = null;
let eventsError: string | null = null;
let eventsLoading = false;
let eventsPaused = false;
let eventsSearch = "";
let eventsSeverityFilter: EventSeverityFilter = "all";
let eventsRequestSequence = 0;
let eventsPollTimer: number | null = null;
let controlStatus: ControlStatusResponse | null = null;
let controlStatusError: string | null = null;
let controlStatusLoading = false;
let controlStatusRequestSequence = 0;
let selectedControlTargetId: string | null = null;
let pendingControl: DeviceControl | null = null;
let pendingControlTargetId: string | null = null;
let pendingControlCancelReason: string | null = null;
let controlConfirmation = "";
let controlBusyAction: ControlAction | null = null;
let controlResult: ControlResult | null = null;
let controlResultTargetId: string | null = null;
let pendingControlLauncherId: string | null = null;
let managementFocusTargetId: string | null = null;
const ledChoices = new Map<string, { desired: number | null; lastRaw: number | null; readAtMs: number | null }>();
let consoleClipboardStatus: string | null = null;
let mcpClipboardStatus: string | null = null;
let autoUpdateChecks = localStorage.getItem("opendrobo.autoUpdateChecks") === "true";
let viewMode: "icons" | "list" = "icons";
let requestGeneration = 0;
let capacityRequestSequence = 0;
let updateRequestSequence = 0;
let capacityExpiryTimer: number | null = null;

function requiredElement(id: string): HTMLElement {
  const element = document.getElementById(id);
  if (!element) throw new Error(`Missing #${id}`);
  return element;
}

function tauriAvailable(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

async function call<T>(command: string, payload?: Record<string, unknown>): Promise<T> {
  if (!tauriAvailable()) throw new Error("Open the desktop application to use local discovery and read-only device commands.");
  return invoke<T>(command, payload);
}

function render(): void {
  const dashboardDomState = captureDashboardDomState();
  if (selectedSection === "status" && poolStatus) {
    const selection = resolveStatusSelection(poolStatus.pools.map((pool) => pool.id), selectedPoolId, selectedControlTargetId);
    selectedControlTargetId = selection.controlTargetId;
    if (selection.changed) {
      clearPendingControl("Displayed Drobo changed; pending control confirmation was cancelled.");
      selectedPoolId = selection.poolId;
      selectedStatusView = "System Information";
      selectedBaySlot = null;
    }
  }
  const consoleDomState = captureConsoleDomState();
  const eventsDomState = captureEventsDomState();
  const managementDomState = captureManagementDomState();
  const shell = element("main", "app-shell");
  shell.append(header(), element("div", "rainbow-rule"));
  shell.append(beautyLineup());
  const body = element("div", "shell-body");
  body.append(sidebar(), panel());
  shell.append(body);
  app.replaceChildren(shell);
  restoreDashboardDomState(dashboardDomState);
  restoreConsoleDomState(consoleDomState);
  restoreEventsDomState(eventsDomState);
  restoreManagementDomState(managementDomState);
  syncConsolePolling();
  syncEventsPolling();
}

function header(): HTMLElement {
  const headerElement = element("header", "topbar");
  const brand = element("div", "brand");
  brand.append(textElement("strong", "OpenDrobo"), textElement("span", "DEVELOPMENT PREVIEW"));
  const selectedPool = currentPool();
  const title = textElement("h1", selectedPool ? displayDeviceLabel(selectedPool.brand, selectedPool.model) : "Drobo Dashboard");
  const metadata = textElement("div", appInfo ? `v${appInfo.version} · ${appInfo.platform}` : "development local dashboard", "top-meta");
  headerElement.append(brand, title, metadata);
  return headerElement;
}

function sidebar(): HTMLElement {
  const aside = element("aside", "sidebar glass");
  const nav = element("nav", "nav-list");
  nav.setAttribute("aria-label", "Dashboard sections");
  const appendSection = (section: typeof sections[number], destination: HTMLElement) => {
    const button = element("button", "nav-item");
    button.type = "button";
    button.classList.toggle("selected", selectedSection === section.id);
    button.setAttribute("aria-current", selectedSection === section.id ? "page" : "false");
    const icon = textElement("span", section.icon, "nav-icon");
    const artworkName = selectedSection === section.id ? section.activeArtwork : section.artwork;
    if (artworkName) {
      const art = document.createElement("img");
      art.className = "nav-art";
      art.src = `/local-artwork/${artworkName}`;
      art.alt = "";
      art.addEventListener("load", () => {
        const visible = navigationCanvas(art);
        if (visible) {
          art.replaceWith(visible);
          icon.classList.add("has-art");
        }
      }, { once: true });
      art.addEventListener("error", () => art.remove(), { once: true });
      icon.append(art);
    }
    const label = section.id === "sharing" ? (appInfo?.platform === "macos" ? "Mac Sharing" : "Host Sharing") : section.label;
    button.append(icon, textElement("span", label));
    button.addEventListener("click", () => selectSection(section.id));
    destination.append(button);
  };
  const groups: Array<[string, string, Section[]]> = [
    ["device", "Device", ["all", "status", "capacity", "volumes", "tools", "settings"]],
    ["activity", "Activity", ["events", "statistics", "console"]],
    ["application", "Host & application", ["sharing", "preferences", "mcp", "management", "help"]]
  ];
  for (const [id, title, ids] of groups) {
    const group = element("div", "nav-group");
    group.setAttribute("role", "group");
    group.setAttribute("aria-labelledby", `navigation-${id}`);
    const heading = textElement("p", title, "nav-group-title");
    heading.id = `navigation-${id}`;
    const items = element("div", "nav-group-items");
    for (const sectionId of ids) {
      const section = sections.find((candidate) => candidate.id === sectionId);
      if (section) appendSection(section, items);
    }
    group.append(heading, items);
    nav.append(group);
  }
  aside.append(nav, productLineup(), footerInfo());
  return aside;
}

function navigationCanvas(image: HTMLImageElement): HTMLCanvasElement | null {
  // Vendor navigation images have large transparent offsets. Fit the glyph,
  // preserving the original local file and falling back if it cannot decode.
  const canvas = document.createElement("canvas");
  const width = image.naturalWidth;
  const height = image.naturalHeight;
  if (!width || !height || width > 512 || height > 512) return null;
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d");
  if (!context) return null;
  try {
    context.drawImage(image, 0, 0);
    const pixels = context.getImageData(0, 0, width, height).data;
    let left = width, top = height, right = -1, bottom = -1;
    for (let y = 0; y < height; y += 1) {
      for (let x = 0; x < width; x += 1) {
        if (pixels[(y * width + x) * 4 + 3] === 0) continue;
        left = Math.min(left, x); top = Math.min(top, y);
        right = Math.max(right, x); bottom = Math.max(bottom, y);
      }
    }
    if (right < left || bottom < top) return null;
    canvas.width = right - left + 1;
    canvas.height = bottom - top + 1;
    context.drawImage(image, left, top, canvas.width, canvas.height, 0, 0, canvas.width, canvas.height);
    canvas.className = "nav-art";
    canvas.setAttribute("aria-hidden", "true");
    return canvas;
  } catch {
    return null;
  }
}

function productLineup(): HTMLElement {
  const lineup = element("div", "lineup");
  lineup.setAttribute("aria-label", "Faded Drobo systems illustration");
  lineup.append(textElement("p", "DROBO SYSTEMS"));
  const units = element("div", "units");
  for (const [index, imageName] of artwork.entries()) {
    const holder = element("div", `tower tower-${index + 1}`);
    const image = document.createElement("img");
    image.src = `/local-artwork/${imageName}`;
    image.alt = "";
    image.loading = "lazy";
    image.addEventListener("error", () => {
      image.remove();
      holder.classList.add("tower-fallback");
      holder.setAttribute("aria-hidden", "true");
    }, { once: true });
    holder.append(image);
    units.append(holder);
  }
  lineup.append(units);
  return lineup;
}

function beautyLineup(): HTMLElement {
  const lineup = element("div", "beauty-lineup");
  lineup.setAttribute("aria-hidden", "true");
  for (const [index, imageName] of artwork.entries()) {
    const holder = element("div", `beauty-unit beauty-unit-${index + 1}`);
    const image = document.createElement("img");
    image.src = `/local-artwork/${imageName}`;
    image.alt = "";
    image.addEventListener("error", () => {
      image.remove();
      holder.classList.add("beauty-fallback");
    }, { once: true });
    holder.append(image);
    lineup.append(holder);
  }
  return lineup;
}

function footerInfo(): HTMLElement {
  const footer = element("div", "sidebar-footer");
  const pool = currentPool();
  const observed = poolStatus ? new Date(poolStatus.observedAtMs).toLocaleTimeString() : "not reported";
  footer.append(textElement("strong", pool ? displayDeviceLabel(pool.brand, pool.model) : poolStatus?.pools.length ? "Select a Drobo" : inventory?.devices.length ? "Device information pending" : "No Drobo detected"), textElement("span", `Last updated: ${observed}`));
  return footer;
}

function panel(): HTMLElement {
  const content = element("section", "content");
  content.id = "main-content";
  content.dataset.section = selectedSection;
  switch (selectedSection) {
    case "all": return allDevicesPanel(content);
    case "status": return statusPanel(content);
    case "capacity": return capacityPanel(content);
    case "volumes": return volumesPanel(content);
    case "events": return eventsPanel(content);
    case "statistics": return statisticsPanel(content);
    case "console": return consolePanel(content);
    case "sharing": return sharingPanel(content);
    case "mcp": return mcpPanel(content);
    case "management": return managementPanel(content);
    case "preferences": return preferencesPanel(content);
    case "tools": return toolsPanel(content);
    case "settings": return settingsPanel(content);
    case "help": return helpPanel(content);
  }
}

function captureDashboardDomState(): { section: Section; scrollTop: number; focusId: string | null; selectionStart: number | null; selectionEnd: number | null; openDetails: number[]; advancedOpen: boolean } | null {
  const content = document.getElementById("main-content");
  if (!content) return null;
  const active = document.activeElement;
  const input = active instanceof HTMLInputElement ? active : null;
  return {
    section: (content.dataset.section as Section) || selectedSection,
    scrollTop: content.scrollTop,
    focusId: active instanceof HTMLElement && active.id ? active.id : null,
    selectionStart: input?.selectionStart ?? null,
    selectionEnd: input?.selectionEnd ?? null,
    openDetails: [...content.querySelectorAll("details")].flatMap((detail, index) => detail.open ? [index] : []),
    advancedOpen: document.querySelector(".nav-advanced") instanceof HTMLDetailsElement && (document.querySelector(".nav-advanced") as HTMLDetailsElement).open
  };
}

function restoreDashboardDomState(state: ReturnType<typeof captureDashboardDomState>): void {
  if (!state || state.section !== selectedSection) return;
  window.requestAnimationFrame(() => {
    if (state.section !== selectedSection) return;
    const content = document.getElementById("main-content");
    if (content) {
      content.scrollTop = state.scrollTop;
      for (const index of state.openDetails) {
        const detail = content.querySelectorAll("details").item(index);
        if (detail) detail.open = true;
      }
    }
    const advanced = document.querySelector(".nav-advanced");
    if (advanced instanceof HTMLDetailsElement) advanced.open = state.advancedOpen;
    if (!state.focusId) return;
    const target = document.getElementById(state.focusId);
    if (!(target instanceof HTMLElement)) return;
    target.focus({ preventScroll: true });
    if (target instanceof HTMLInputElement && state.selectionStart !== null && state.selectionEnd !== null) target.setSelectionRange(state.selectionStart, state.selectionEnd);
  });
}

function allDevicesPanel(content: HTMLElement): HTMLElement {
  const intro = panelIntro("All Drobos", "Select a Drobo to view its status, capacity and drives.");
  const actions = element("div", "panel-actions");
  const refresh = actionButton((inventoryLoading || poolLoading) ? "Refreshing…" : "Refresh devices", "primary", () => { void loadInventory(); void loadPoolStatus(true); });
  refresh.disabled = inventoryLoading || poolLoading;
  actions.append(refresh, segmentedControl("Device view", [
    ["Icons", viewMode === "icons", () => { viewMode = "icons"; render(); }],
    ["List", viewMode === "list", () => { viewMode = "list"; render(); }]
  ]));
  intro.append(actions);
  content.append(intro);

  const pools = poolStatus?.pools ?? [];
  if (pools.length > 0) {
    const list = element("div", viewMode === "icons" ? "device-grid" : "device-list");
    for (const pool of pools) list.append(poolDeviceCard(pool));
    content.append(list);
  } else {
    content.append(stateCard(poolLoading ? "Reading Drobo information" : "Device information unavailable", poolLoading ? "Waiting for device details." : "Refresh devices to try again. Connection details are available under Advanced discovery.", "quiet"));
  }
  const discovery = document.createElement("details");
  discovery.className = "advanced-details";
  discovery.append(textElement("summary", "Advanced discovery"));
  if (inventoryError) discovery.append(liveStatus("Registry unavailable", inventoryError, "error"));
  else if (!inventory) discovery.append(textElement("p", "Checking local registry.", "status-copy"));
  else if (inventory.devices.length === 0) discovery.append(textElement("p", inventory.message || "No endpoints reported.", "status-copy"));
  else {
    const list = element("div", "device-list");
    for (const device of [...inventory.devices].sort((a, b) => a.name.localeCompare(b.name))) list.append(deviceCard(device));
    discovery.append(list, textElement("p", inventory.message, "status-copy"));
  }
  content.append(discovery);
  content.append(statusStrip());
  return content;
}

function statusPanel(content: HTMLElement): HTMLElement {
  content.append(panelIntro("Status", "Device and drive information."));
  if (poolLoading && !poolStatus) {
    content.append(liveStatus("Reading device snapshot", "Requesting the local original-service snapshot.", "quiet"));
    return content;
  }
  if (poolError && !poolStatus) {
    content.append(liveStatus("Device snapshot unavailable", poolError, "error"));
    return content;
  }
  if (!poolStatus || poolStatus.pools.length === 0) {
    content.append(stateCard("No device snapshot", "Read shared pool from All Drobos or Capacity to inspect reported model, unit presence, and slots.", "quiet"));
    return content;
  }
  if (poolStatus.pools.length > 1) content.append(statusDevicePicker(poolStatus.pools));
  if (poolIsStale()) content.append(liveStatus("Device snapshot stale", poolStaleError ?? "The original-service snapshot is older than 30 seconds.", "quiet"));
  const pool = currentPool();
  if (!pool) { content.append(stateCard("Choose a Drobo", "Select an enclosure above to view its system information.", "quiet")); return content; }
  const views: Array<typeof selectedStatusView> = ["System Information", "Drive Information"];
  const selector = element("div", "status-view-options");
  selector.append(segmentedControl("Device information views", views.map((name): [string, boolean, () => void] => [name, selectedStatusView === name, () => { selectedStatusView = name; render(); }])));
  const details = element("section", "status-details glass");
  if (selectedStatusView === "System Information") details.append(systemInformation(pool), statusAdvancedDetails(pool));
  else if (selectedStatusView === "Drive Information") details.append(driveInformation(pool));
  const enclosure = enclosureArtwork(pool, !poolIsStale());
  const layout = element("div", "status-workspace"); layout.append(details, enclosure);
  content.append(selector, layout, textElement("p", isFiveD(pool) ? "Drive colors follow reported Drobo LED codes. Gray means unavailable or stale. Selection glow is separate from drive status." : "Reported slot presence is shown; LED decoding is not verified for this model.", "scope-note"));
  return content;
}

function statusDevicePicker(pools: PoolStatus[]): HTMLElement {
  const picker = element("div", "status-device-picker");
  picker.setAttribute("role", "group");
  picker.setAttribute("aria-label", "Choose a Drobo for Status");
  for (const pool of pools) {
    const button = element("button", "status-device-choice glass");
    button.type = "button";
    button.id = `status-device-${encodeURIComponent(pool.id)}`;
    button.setAttribute("aria-pressed", String(selectedPoolId === pool.id));
    const label = displayDeviceLabel(pool.brand, pool.model);
    const identity = statusUnitLabel(pool.id, pools.map((candidate) => candidate.id));
    button.setAttribute("aria-label", `${label}, ${identity}`);
    button.append(unitPortrait(pool, !poolIsStale()), textElement("strong", label), textElement("small", identity));
    button.addEventListener("click", () => {
      clearPendingControl("Displayed Drobo changed; pending control confirmation was cancelled.");
      if (selectedPoolId !== pool.id) selectedControlTargetId = null;
      selectedPoolId = pool.id;
      selectedStatusView = "System Information";
      selectedBaySlot = null;
      render();
    });
    picker.append(button);
  }
  return picker;
}

function currentPool(): PoolStatus | null {
  const pools = poolStatus?.pools ?? [];
  return pools.find((pool) => pool.id === selectedPoolId) ?? null;
}

function poolSelector(id: string): HTMLLabelElement {
  const label = textElement("label", "Selected Drobo", "field-label");
  const select = document.createElement("select");
  select.id = id;
  select.className = "device-select";
  const placeholder = new Option("Choose a Drobo…", "");
  placeholder.disabled = true;
  placeholder.selected = selectedPoolId === null;
  select.append(placeholder);
  for (const pool of poolStatus?.pools ?? []) {
    const option = new Option(displayDeviceLabel(pool.brand, pool.model), pool.id);
    option.selected = pool.id === selectedPoolId;
    select.append(option);
  }
  select.addEventListener("change", () => {
    if (!poolStatus?.pools.some((pool) => pool.id === select.value)) return;
    clearPendingControl("Selected Drobo changed; pending power confirmation was cancelled.");
    selectedPoolId = select.value;
    selectedControlTargetId = select.value;
    selectedBaySlot = null;
    render();
  });
  label.htmlFor = id;
  label.append(select);
  return label;
}

function displayDeviceLabel(brand: string, model: string): string {
  const cleanBrand = brand.trim(); const cleanModel = model.trim();
  return cleanBrand && cleanModel.toLocaleLowerCase().startsWith(cleanBrand.toLocaleLowerCase()) ? cleanModel : [cleanBrand, cleanModel].filter(Boolean).join(" ") || "Drobo";
}

function poolDeviceCard(pool: PoolStatus): HTMLElement {
  const card = element("article", "device-card glass");
  card.classList.toggle("device-selected", selectedPoolId === pool.id);
  card.append(unitPortrait(pool, !poolIsStale()), textElement("h2", displayDeviceLabel(pool.brand, pool.model)));
  card.append(textElement("p", `Capacity ${formatDecimalBytes(pool.protectedUsedBytes)} used of ${formatDecimalBytes(pool.protectedTotalBytes)}. ${poolIsStale() ? "Snapshot stale." : "Snapshot current."}`, "status-copy"));
  const select = actionButton("Open status", "secondary", () => { if (selectedPoolId !== pool.id) clearPendingControl("Pool changed; pending control confirmation was cancelled."); selectedPoolId = pool.id; selectedControlTargetId = pool.id; selectedBaySlot = null; selectedSection = "status"; requestGeneration += 1; render(); });
  select.id = `pool-${encodeURIComponent(pool.id)}`;
  card.append(select);
  return card;
}

function systemInformation(pool: PoolStatus): HTMLElement {
  const list = element("dl", "info-list");
  appendDefinition(list, "Brand", pool.brand || "Not reported");
  appendDefinition(list, "Model", pool.model || "Not reported");
  appendDefinition(list, "Firmware", pool.firmwareVersion ?? "Not reported");
  appendDefinition(list, "Firmware release date", pool.firmwareReleaseDate ?? "Not reported");
  appendDefinition(list, "Last updated", poolStatus ? new Date(poolStatus.observedAtMs).toLocaleString() : "Not reported");
  appendDefinition(list, "Unit presence", poolIsStale() ? "Unknown — snapshot stale" : observationStateCopy(pool.unitState));
  return list;
}

function driveInformation(pool: PoolStatus): HTMLElement {
  const drives = dataSlots(pool);
  const selected = (selectedBaySlot === 5 ? cacheSlot(pool) : null) ?? drives.find((slot) => slot.slot === selectedBaySlot) ?? drives[0];
  if (!selected) return textElement("p", "No data-drive detail reported.", "status-copy");
  const wrap = element("div", "drive-info");
  const previous = actionButton("Previous", "secondary", () => selectDrive(pool, selected.slot, -1));
  const next = actionButton("Next", "secondary", () => selectDrive(pool, selected.slot, 1));
  const list = element("dl", "info-list");
  appendDefinition(list, "Drive", selected.slot === 5 && isFiveD(pool) ? "mSATA cache" : isFiveD(pool) ? `Drive ${selected.slot + 1}` : `Reported slot ${selected.slot}`);
  appendDefinition(list, "Make / model", selected.manufacturerModel ?? "Not reported");
  appendDefinition(list, "Firmware revision", selected.firmwareRevision ?? "Not reported");
  appendDefinition(list, "Kind", cacheSlot(pool)?.slot === selected.slot ? "mSATA cache SSD" : selected.kind === "ssd" ? "SSD" : "Data drive");
  appendDefinition(list, "Capacity", formatPhysicalCapacity(selected.physicalCapacityBytes));
  appendDefinition(list, "SSD life", selected.ssdLifeRemainingPercent === null ? "Not reported" : `${selected.ssdLifeRemainingPercent}% remaining`);
  appendDefinition(list, "Presence", poolIsStale() ? "Unknown — last observation is stale" : observationStateCopy(selected.driveState));
  if (isFiveD(pool) && selected.slot < 5) appendDefinition(list, "Drive indicator", driveIndicator(selected.rawStatus.mStatus, !poolIsStale()).label);
  if (cacheSlot(pool)?.slot === selected.slot) appendDefinition(list, "Cache usage", "Not reported");
  wrap.append(element("div", "panel-actions"), list);
  (wrap.firstElementChild as HTMLElement).append(previous, next);
  return wrap;
}

function selectDrive(pool: PoolStatus, slot: number, direction: -1 | 1): void {
  const cache = cacheSlot(pool);
  const drives = [...dataSlots(pool), ...(cache ? [cache] : [])];
  const index = Math.max(0, drives.findIndex((item) => item.slot === slot));
  selectedBaySlot = drives[(index + direction + drives.length) % drives.length]?.slot ?? null;
  render();
}

function enclosureArtwork(pool: PoolStatus, fresh: boolean): HTMLElement {
  const wrap = element("section", "enclosure-card glass");
  wrap.append(textElement("h3", "Front enclosure", "card-heading"));
  if (!isFiveD(pool)) {
    wrap.append(textElement("p", "Generic reported-slot view. Original 5D bay geometry is not assumed for this model.", "status-copy"));
    const list = element("div", "reported-slots");
    for (const slot of dataSlots(pool)) {
      const button = actionButton(`Slot ${slot.slot}`, "secondary", () => { selectedBaySlot = slot.slot; selectedStatusView = "Drive Information"; render(); });
      button.classList.toggle("present", fresh && slot.driveState === "active"); list.append(button);
    }
    wrap.append(list); return wrap;
  }
  const stage = element("div", "front-stage");
  const chassis = element("div", "front-chassis");
  chassis.setAttribute("aria-hidden", "true");
  chassis.append(textElement("span", "OpenDrobo", "chassis-wordmark"));
  const art = document.createElement("img");
  art.src = "/local-artwork/Drobo5DFront.png";
  art.alt = "";
  art.className = "front-art";
  art.addEventListener("load", () => stage.classList.add("has-original-art"), { once: true });
  art.addEventListener("error", () => art.remove(), { once: true });
  stage.append(chassis, art);
  const bays = element("div", "front-bays");
  const slots = dataSlots(pool);
  for (let index = 0; index < 5; index += 1) {
    const slot = slots.find((item) => item.slot === index);
    const bay = actionButton(`Drive ${index + 1}`, "secondary", () => {
      if (!slot) return;
      selectedBaySlot = slot.slot;
      selectedStatusView = "Drive Information";
      render();
    });
    bay.classList.add("front-bay");
    bay.id = `bay-${index}`;
    bay.style.setProperty("--bay-index", String(index));
    bay.disabled = !slot;
    bay.classList.toggle("present", Boolean(slot && fresh && slot.driveState === "active"));
    const indicator = driveIndicator(slot?.rawStatus.mStatus, fresh);
    bay.dataset.led = indicator.kind;
    bay.title = indicator.label;
    const selected = selectedStatusView === "Drive Information" && slot?.slot === (selectedBaySlot ?? slots[0]?.slot);
    bay.classList.toggle("bay-selected", selected);
    bay.setAttribute("aria-pressed", String(selected));
    bay.setAttribute("aria-label", `Drive ${index + 1}: ${indicator.label}; ${fresh && slot ? observationStateCopy(slot.driveState) : "current presence unknown"}${slot?.manufacturerModel ? `, ${slot.manufacturerModel}` : ""}`);
    bay.addEventListener("keydown", (event) => {
      if (!slot || (event.key !== "ArrowDown" && event.key !== "ArrowUp")) return;
      event.preventDefault();
      selectedStatusView = "Drive Information";
      selectedBaySlot = slot.slot;
      selectDrive(pool, slot.slot, event.key === "ArrowDown" ? 1 : -1);
      window.requestAnimationFrame(() => document.getElementById(selectedBaySlot === 5 ? "cache-slot" : `bay-${selectedBaySlot}`)?.focus());
    });
    bays.append(bay);
  }
  const cache = cacheSlot(pool);
  const cacheButton = actionButton(cache ? `mSATA cache · ${formatPhysicalCapacity(cache.physicalCapacityBytes)}` : "mSATA cache · not reported", "secondary", () => { if (!cache) return; selectedBaySlot = 5; selectedStatusView = "Drive Information"; render(); });
  cacheButton.id = "cache-slot";
  cacheButton.classList.add("cache-select");
  cacheButton.setAttribute("aria-pressed", String(selectedStatusView === "Drive Information" && selectedBaySlot === 5));
  cacheButton.disabled = !cache;
  stage.append(bays);
  const legend = element("div", "drive-led-legend");
  legend.setAttribute("aria-label", "Drive indicator colors");
  for (const [kind, label] of [["green", "Green"], ["amber", "Amber"], ["red", "Red"], ["protecting", "Green / amber"], ["red-flash", "Flashing red"], ["unknown", "Unknown"]]) {
    const item = textElement("span", label); item.dataset.led = kind; legend.append(item);
  }
  wrap.append(stage, cacheButton, textElement("p", "Select a drive to view its details.", "status-copy"), legend);
  return wrap;
}

function isFiveD(pool: PoolStatus): boolean {
  return pool.model === "Drobo 5D" || pool.model === "5D";
}

function dataSlots(pool: PoolStatus): PoolSlot[] {
  return isFiveD(pool) ? pool.slots.filter((slot) => slot.slot >= 0 && slot.slot <= 4).sort((a, b) => a.slot - b.slot) : [...pool.slots].sort((a, b) => a.slot - b.slot);
}

function cacheSlot(pool: PoolStatus): PoolSlot | null {
  return isFiveD(pool) ? pool.slots.find((slot) => slot.slot === 5 && slot.kind === "ssd") ?? null : null;
}

function statusAdvancedDetails(pool: PoolStatus): HTMLElement {
  const details = document.createElement("details");
  details.className = "advanced-details";
  details.append(textElement("summary", "Advanced technical details"), textElement("p", `Raw device state: ${formatRawDeviceStatus(pool.deviceStatus)}`, "status-copy"), poolLunTable(pool), poolSlotTable(pool));
  for (const slot of pool.slots) details.append(textElement("p", `Slot ${slot.slot} raw state: ${formatRawSlotStatus(slot.rawStatus)}`, "status-copy"));
  return details;
}

function eventsPanel(content: HTMLElement): HTMLElement {
  const intro = panelIntro("Events", "History begins when OpenDrobo observes this device. It is not recovered onboard event history or a system log.");
  const actions = element("div", "panel-actions");
  const refresh = actionButton(eventsLoading ? "Refreshing…" : "Refresh", "primary", () => void loadDeviceEvents(true));
  refresh.id = "events-refresh";
  refresh.disabled = eventsLoading;
  const pause = actionButton(eventsPaused ? "Resume polling" : "Pause polling", "secondary", () => {
    eventsPaused = !eventsPaused;
    render();
  });
  pause.id = "events-pause";
  actions.append(refresh, pause);
  intro.append(actions);
  content.append(intro);

  if (!eventsResponse && !eventsLoading && !eventsError) window.queueMicrotask(() => void loadDeviceEvents(true));
  if (eventsLoading && !eventsResponse) content.append(liveStatus("Loading observed events", "Reading a bounded local observation history.", "quiet"));
  if (eventsError) content.append(liveStatus("Events unavailable", eventsError, "error"));
  if (!eventsResponse) return content;

  const toolbar = element("div", "console-toolbar glass");
  const searchLabel = textElement("label", "Search events", "field-label");
  searchLabel.setAttribute("for", "events-search");
  const search = document.createElement("input");
  search.id = "events-search";
  search.className = "console-search";
  search.type = "search";
  search.placeholder = "Source, unit, or message";
  search.value = eventsSearch;
  search.addEventListener("input", () => {
    eventsSearch = search.value;
    render();
  });
  searchLabel.append(search);
  const severityLabel = textElement("label", "Severity", "field-label");
  severityLabel.setAttribute("for", "events-severity");
  const severity = document.createElement("select");
  severity.id = "events-severity";
  severity.className = "device-select";
  for (const optionValue of ["all", "info", "warning", "error"] as EventSeverityFilter[]) {
    const option = document.createElement("option");
    option.value = optionValue;
    option.textContent = optionValue === "all" ? "All severities" : optionValue[0].toUpperCase() + optionValue.slice(1);
    option.selected = eventsSeverityFilter === optionValue;
    severity.append(option);
  }
  severity.addEventListener("change", () => {
    eventsSeverityFilter = severity.value as EventSeverityFilter;
    render();
  });
  severityLabel.append(severity);
  toolbar.append(searchLabel, severityLabel);
  content.append(toolbar);
  content.append(eventsObservationSummary(eventsResponse));

  const records = filteredDeviceEvents();
  const events = element("section", "console-events glass");
  events.id = "device-events";
  events.tabIndex = 0;
  events.setAttribute("role", "region");
  events.setAttribute("aria-label", "Observed device events");
  events.setAttribute("aria-live", "off");
  if (records.length === 0) {
    events.append(textElement("p", "No observed events match the current filters.", "status-copy"));
  } else {
    for (const record of records) events.append(deviceEventRecord(record));
  }
  const observedSince = eventsResponse.observedSinceMs === null ? "Observation start not reported" : `Observed since ${formatConsoleTime(eventsResponse.observedSinceMs)}`;
  content.append(events, textElement("p", `${eventsResponse.message} ${observedSince}. ${eventsPaused ? "Polling paused." : "Polling every 2 seconds while this Events section is visible."}`, "scope-note"));
  return content;
}

function eventsObservationSummary(response: DeviceEventsResponse): HTMLElement {
  const summary = element("section", "event-observation glass");
  const observed = response.lastObservedAtMs === null ? "not reported" : formatConsoleTime(response.lastObservedAtMs);
  summary.append(textElement("h3", "Observation state", "card-heading"), textElement("p", `${observationStateCopy(response.state)}. Last observed: ${observed}.`, "status-copy"));
  if (response.units.length === 0) {
    summary.append(textElement("p", "No unit-presence records reported by the event engine.", "status-copy"));
    return summary;
  }
  const units = element("div", "observation-units");
  for (const unit of response.units) {
    const item = element("div", "observation-unit");
    const matchingPool = poolStatus?.pools.find((pool) => pool.id === unit.id);
    const unitLabel = `${matchingPool?.model || "Drobo unit"} · ${opaqueReferenceSuffix(unit.id)}`;
    item.title = unit.id;
    const lastObserved = unit.lastObservedAtMs === null ? "not reported" : formatConsoleTime(unit.lastObservedAtMs);
    const slots = unit.slots.length === 0 ? "No slot states reported" : unit.slots.map((slot) => `Slot ${slot.slot}: ${slot.state}`).join(" · ");
    item.append(textElement("span", unit.state, `service-state ${unit.state}`), textElement("strong", unitLabel), textElement("p", `${observationStateCopy(unit.state)}. Last observed: ${lastObserved}. ${slots}.`, "status-copy"));
    units.append(item);
  }
  summary.append(units);
  return summary;
}

function opaqueReferenceSuffix(reference: string): string {
  const suffix = reference.split("/").at(-1)?.slice(-8);
  return suffix && suffix.length === 8 ? suffix : "unknown";
}

function deviceEventRecord(record: DeviceEvent): HTMLElement {
  const row = element("article", `console-record ${record.severity}`);
  const meta = element("div", "console-record-meta");
  const unit = record.unitId ? `Unit ${record.unitId}` : "No unit reported";
  const raw = record.rawHealthState === null ? "Raw health state: not reported" : `Raw health state: ${record.rawHealthState}`;
  const location = record.slot === null || record.slot === undefined ? null : `Slot ${record.slot}`;
  meta.append(textElement("time", formatConsoleTime(record.timestampMs)), textElement("span", record.severity, `console-level ${record.severity}`), textElement("span", record.source), textElement("span", unit), textElement("span", raw));
  if (record.kind) meta.append(textElement("span", record.kind));
  if (location) meta.append(textElement("span", location));
  row.append(meta, textElement("p", record.message));
  return row;
}

function managementPanel(content: HTMLElement): HTMLElement {
  content.append(panelIntro("Reference", "Read-only capability catalogue. It does not determine per-unit availability or send actions."));
  const inventory = element("section", "management-grid");
  const controls: Array<{ category: string; name: string; state: "available" | "research" | "unsupported"; message: string }> = [
    { category: "Monitoring", name: "Pool and device snapshot", state: "available", message: "Local original-service snapshot only; no control action is sent." },
    { category: "Device preferences", name: "Identify / blink", state: "research", message: "Current per-unit availability is reported in the allowlisted controls above; broader transport coverage remains under research." },
    { category: "Device preferences", name: "LED brightness and night mode", state: "research", message: "Current per-unit availability is reported in the allowlisted controls above; broader transport coverage remains under research." },
    { category: "Device preferences", name: "Restart, shutdown, and standby", state: "research", message: "Current per-unit availability is reported in the allowlisted controls above; broader transport coverage remains under research." },
    { category: "Device preferences", name: "Rename device", state: "research", message: "Requires exact device-setting semantics and a confirmed target state." },
    { category: "Device preferences", name: "Dual disk redundancy", state: "research", message: "Requires exact configuration semantics and a confirmed state model." },
    { category: "Device preferences", name: "Drive spindown", state: "research", message: "Requires exact configuration semantics and a confirmed state model." },
    { category: "Storage and volumes", name: "Create, delete, and rename volumes", state: "unsupported", message: "Volume mutation is not implemented." },
    { category: "Storage and volumes", name: "Mount and unmount volumes", state: "unsupported", message: "OpenDrobo does not mount or unmount volumes." },
    { category: "Storage and volumes", name: "Format and reset", state: "unsupported", message: "Destructive storage actions are intentionally not implemented." },
    { category: "Storage and volumes", name: "CHAP configuration", state: "unsupported", message: "Authentication configuration is not implemented." },
    { category: "Notifications and updates", name: "Email and notifications", state: "research", message: "Requires an exact local configuration protocol; no account data is stored here." },
    { category: "Notifications and updates", name: "Firmware update", state: "unsupported", message: "Firmware writes are intentionally not implemented." },
    { category: "NAS administration", name: "NAS users and shares", state: "unsupported", message: "Model-specific NAS administration is not implemented." },
    { category: "NAS administration", name: "NAS time and administrator settings", state: "unsupported", message: "Model-specific NAS administration is not implemented." },
    { category: "Network", name: "LAN automatic and manual discovery", state: "unsupported", message: "Network discovery configuration is not implemented." },
    { category: "Extensions", name: "DroboSync and DroboApps", state: "unsupported", message: "NAS extension management is not implemented." },
    { category: "Hardware", name: "Performance sensors, fans, power, and battery", state: "research", message: "Model-specific hardware telemetry remains under research." },
    { category: "Diagnostics", name: "Diagnostic and support exports", state: "unsupported", message: "Support export tooling is not implemented; no shell or credential tooling is exposed." }
  ];
  let category = "";
  for (const item of controls) {
    if (item.category !== category) {
      category = item.category;
      inventory.append(textElement("h3", category, "management-category"));
    }
    const card = element("article", "management-card glass");
    card.append(textElement("span", item.state, `management-state ${item.state}`), textElement("h3", item.name), textElement("p", item.message, "status-copy"));
    if (item.state !== "available") {
      const control = actionButton(item.state === "research" ? "Catalogue item" : "Not supported", "secondary", () => undefined);
      control.disabled = true;
      card.append(control);
    }
    inventory.append(card);
  }
  content.append(textElement("p", "Reference only. Controls live in Tools; no catalogue card sends an action.", "scope-note"), inventory);
  return content;
}

function controlStatusPanel(): HTMLElement {
  const panel = element("section", "control-panel");
  const heading = element("div", "control-heading");
  heading.append(textElement("h3", "Device Operations", "card-heading"));
  const refresh = actionButton(controlStatusLoading ? "Refreshing…" : "Refresh controls", "primary", () => void loadControlStatus());
  refresh.disabled = controlStatusLoading;
  heading.append(refresh);
  panel.append(heading);
  if (!controlStatus && !controlStatusLoading && !controlStatusError) window.queueMicrotask(() => void loadControlStatus());
  if (controlStatusLoading && !controlStatus) {
    panel.append(liveStatus("Checking local controls", "Reading the backend-reported control allowlist and safety state.", "quiet"));
    return panel;
  }
  if (controlStatusError) {
    panel.append(liveStatus("Control status unavailable", controlStatusError, "error"));
    return panel;
  }
  if (!controlStatus || controlStatus.targets.length === 0) {
    panel.append(stateCard("No local controls available", controlStatus?.message ?? "Desktop runtime required to inspect control availability.", "quiet"));
    return panel;
  }
  if (controlStatus.state === "unavailable") panel.append(liveStatus("Local controls unavailable", controlStatus.message, "quiet"));
  if (controlStatus.targets.length > 1 || !selectedControlTarget()) panel.append(controlTargetSelect(controlStatus.targets));
  const target = selectedControlTarget();
  if (!target) {
    panel.append(stateCard("Selected device unavailable", "Refresh device information before using controls for this Drobo.", "quiet"));
    return panel;
  }
  panel.append(textElement("p", `${displayDeviceLabel(target.brand, target.model)} · Experimental controls. Power operations require all Drobo volumes to be ejected.`, "status-copy"));
  const controls = element("div", "control-grid");
  const order: ControlAction[] = ["identify", "shutdown", "standby", "restart", "get-brightness", "brightness"];
  const deviceControls = target.controls.filter((control) => control.action !== "brightness" && control.action !== "get-brightness");
  for (const control of [...deviceControls].sort((left, right) => order.indexOf(left.action) - order.indexOf(right.action))) {
    controls.append(controlCard(target, control));
    if (control.action === "identify") controls.append(unavailableControlCard("Rename", "Disabled — device-name write is not implemented."));
  }
  panel.append(controls);
  if (pendingControl && pendingControlTargetId === target.id) panel.append(controlConfirmationDialog(target, pendingControl));
  if (pendingControlCancelReason) {
    panel.append(liveStatus("Confirmation cancelled", pendingControlCancelReason, "quiet"));
    pendingControlCancelReason = null;
  }
  if (controlResult) panel.append(controlResultCard(controlResult));
  return panel;
}

function unavailableControlCard(label: string, reason: string): HTMLElement {
  const card = element("article", "control-card glass");
  const button = actionButton(label, "secondary", () => undefined); button.disabled = true;
  card.append(textElement("span", "unavailable", "management-state unsupported"), textElement("h4", label), textElement("p", reason, "status-copy"), button);
  return card;
}

function controlTargetSelect(targets: ControlTarget[]): HTMLElement {
  const label = textElement("label", "Control target", "field-label");
  label.setAttribute("for", "control-target");
  const select = document.createElement("select");
  select.id = "control-target";
  select.className = "device-select";
  const placeholder = new Option("Choose a Drobo…", "");
  placeholder.disabled = true;
  placeholder.selected = selectedControlTarget() === null;
  select.append(placeholder);
  for (const target of targets) {
    const option = document.createElement("option");
    option.value = target.id;
    option.textContent = `${displayDeviceLabel(target.brand, target.model)} · ${target.id.slice(-6)}`;
    option.selected = target.id === selectedControlTargetId && target.id === selectedPoolId;
    select.append(option);
  }
  select.addEventListener("change", () => {
    if (!targets.some((target) => target.id === select.value)) return;
    selectedControlTargetId = select.value;
    selectedPoolId = select.value;
    clearPendingControl();
    controlResult = null;
    render();
  });
  label.append(select);
  return label;
}

function selectedControlTarget(): ControlTarget | null {
  const targets = controlStatus?.targets ?? [];
  if (selectedControlTargetId === null || selectedPoolId !== selectedControlTargetId) return null;
  return targets.find((target) => target.id === selectedControlTargetId) ?? null;
}

function controlCard(target: ControlTarget, control: DeviceControl): HTMLElement {
  const card = element("article", "control-card glass");
  const state = control.enabled ? "available" : "unavailable";
  const labelText = control.action === "identify" ? "Blink Lights" : control.action === "brightness" ? "LED preset" : control.action === "get-brightness" ? "Read LED preset" : control.label;
  card.append(textElement("span", state, `management-state ${state}`), textElement("h4", labelText), textElement("p", control.action === "brightness" ? `${control.reason} Presets 0–9 map to original raw scale 1, 2, 5, 7, 9, 11, 15, 25, 40, 59; this is not a percentage.` : control.action === "get-brightness" ? `${control.reason} The returned value is the original raw dim scale, not a percentage.` : control.reason, "status-copy"));
  if (control.action === "brightness") {
    const rangeAvailable = control.minimum !== null && control.maximum !== null;
    const label = textElement("label", rangeAvailable ? `LED preset (${control.minimum}–${control.maximum})` : "LED preset range unavailable", "field-label");
    const input = document.createElement("input");
    input.type = "number";
    input.id = `brightness-${target.id}`;
    input.value = "";
    input.placeholder = rangeAvailable ? `Choose ${control.minimum}–${control.maximum}` : "Unavailable";
    if (control.minimum !== null) input.min = String(control.minimum);
    if (control.maximum !== null) input.max = String(control.maximum);
    input.disabled = !control.enabled || !rangeAvailable || controlBusyAction !== null;
    label.append(input);
    const submit = actionButton("Set LED preset", "secondary", () => {
      const raw = input.value.trim();
      if (!raw) return;
      void executeControl(target.id, control, Number(raw));
    });
    submit.disabled = true;
    input.addEventListener("input", () => {
      const value = Number(input.value);
      submit.disabled = !control.enabled || !rangeAvailable || controlBusyAction !== null || input.value.trim() === "" || !Number.isInteger(value) || value < (control.minimum ?? 0) || value > (control.maximum ?? 0);
    });
    card.append(label, submit);
    return card;
  }
  const needsTypedConfirmation = control.requiresConfirmation && isPowerAction(control.action);
  const button = actionButton(controlBusyAction === control.action ? `${labelText}…` : labelText, "secondary", () => {
    if (needsTypedConfirmation) {
      pendingControl = control;
      pendingControlTargetId = target.id;
      pendingControlLauncherId = button.id;
      managementFocusTargetId = "control-confirmation";
      controlConfirmation = "";
      controlResult = null;
      render();
      return;
    }
    void executeControl(target.id, control);
  });
  button.id = controlLauncherId(target.id, control.action);
  button.disabled = !control.enabled || controlBusyAction !== null;
  card.append(button);
  return card;
}

function controlConfirmationDialog(target: ControlTarget, control: DeviceControl): HTMLElement {
  const dialog = element("section", "control-confirmation glass");
  dialog.setAttribute("role", "dialog");
  dialog.setAttribute("aria-modal", "false");
  dialog.setAttribute("aria-labelledby", "control-confirmation-heading");
  const required = control.action.toUpperCase();
  const heading = textElement("h3", `Confirm ${control.label}`);
  heading.id = "control-confirmation-heading";
  dialog.append(heading, textElement("p", `This can disrupt access. Eject all volumes before continuing. Type ${required} to submit ${control.label} for ${displayDeviceLabel(target.brand, target.model)}.`, "status-copy"));
  if (controlBusyAction === control.action) {
    const busy = textElement("p", `${control.label} request in progress. Awaiting the local control result.`, "control-busy-status");
    busy.id = "control-confirmation-status";
    busy.tabIndex = -1;
    busy.setAttribute("role", "status");
    busy.setAttribute("aria-live", "polite");
    dialog.append(busy);
  }
  const label = textElement("label", `Type ${required} to confirm`, "field-label");
  label.setAttribute("for", "control-confirmation");
  const input = document.createElement("input");
  input.id = "control-confirmation";
  input.type = "text";
  input.autocomplete = "off";
  input.value = controlConfirmation;
  input.disabled = controlBusyAction !== null;
  let submit: HTMLButtonElement;
  input.addEventListener("input", () => {
    controlConfirmation = input.value;
    submit.disabled = controlConfirmation !== required || controlBusyAction !== null;
  });
  label.append(input);
  const actions = element("div", "panel-actions");
  const cancel = actionButton("Cancel", "secondary", () => {
    clearPendingControl();
    managementFocusTargetId = pendingControlLauncherId;
    pendingControlLauncherId = null;
    render();
  });
  cancel.disabled = controlBusyAction !== null;
  submit = actionButton(controlSubmitLabel(control.action), "primary", () => void executeControl(target.id, control, undefined, controlConfirmation));
  submit.id = `control-confirm-submit-${control.action}`;
  submit.disabled = controlConfirmation !== required || controlBusyAction !== null;
  actions.append(cancel, submit);
  dialog.append(label, actions);
  return dialog;
}

function controlResultCard(result: ControlResult): HTMLElement {
  const title = result.state === "accepted" ? `${result.action} accepted` : result.state === "rejected" ? `${result.action} rejected` : `${result.action} result unknown`;
  const message = result.state === "unknown" ? `${result.message} Do not retry automatically; refresh control status before deciding what to do next.` : result.message;
  const card = stateCard(title, message, result.state === "accepted" ? "quiet" : "error");
  card.id = "control-result-status";
  card.setAttribute("role", "status");
  card.setAttribute("aria-live", "polite");
  card.setAttribute("aria-atomic", "true");
  if (result.value !== null && result.value !== undefined) {
    const valueLabel = result.action === "get-brightness" ? `Reported raw dim value (not a percentage): ${result.value}` : `Reported value: ${result.value}`;
    card.append(textElement("p", valueLabel, "status-copy"));
  }
  const target = controlStatus?.targets.find((candidate) => candidate.id === controlResultTargetId);
  if (target) card.append(textElement("p", `Target: ${displayDeviceLabel(target.brand, target.model)}.`, "status-copy"));
  return card;
}

function isPowerAction(action: ControlAction): boolean {
  return action === "restart" || action === "shutdown" || action === "standby";
}

function controlSubmitLabel(action: ControlAction): string {
  if (action === "restart") return "Restart unit";
  if (action === "shutdown") return "Shutdown unit";
  if (action === "standby") return "Standby unit";
  return "Submit control";
}

function controlLauncherId(targetId: string, action: ControlAction): string {
  return `control-${action}-${encodeURIComponent(targetId)}`;
}

function clearPendingControl(reason?: string): void {
  const hadPending = pendingControl !== null;
  pendingControl = null;
  pendingControlTargetId = null;
  controlConfirmation = "";
  if (hadPending && reason) pendingControlCancelReason = reason;
}

function deviceArtwork(pool: PoolStatus, fresh: boolean): SVGSVGElement {
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("class", "device-artwork");
  svg.setAttribute("viewBox", "0 0 220 280");
  svg.setAttribute("role", "img");
  svg.setAttribute("aria-label", `${pool.model || "Drobo"} illustration. Cyan indicates observed presence, not drive health.`);
  const shape = (name: "rect" | "circle", attributes: Record<string, string>) => {
    const node = document.createElementNS("http://www.w3.org/2000/svg", name);
    for (const [key, value] of Object.entries(attributes)) node.setAttribute(key, value);
    svg.append(node);
  };
  shape("rect", { x: "22", y: "8", width: "176", height: "242", rx: "18", class: "tower-body" });
  shape("rect", { x: "32", y: "20", width: "156", height: "192", rx: "12", class: "tower-bezel" });
  if (isFiveD(pool)) {
    for (let index = 0; index < 5; index += 1) {
      const slot = pool.slots.find((candidate) => candidate.slot === index);
      const active = fresh && slot?.driveState === "active";
      shape("rect", { x: "42", y: String(31 + index * 34), width: "124", height: "25", rx: "4", class: "tower-bay" });
      shape("rect", { x: "47", y: String(40 + index * 34), width: "17", height: "6", rx: "2", class: "tower-latch" });
      shape("circle", { cx: "177", cy: String(44 + index * 34), r: "3", class: active ? "tower-light active" : "tower-light" });
    }
  } else {
    const title = document.createElementNS("http://www.w3.org/2000/svg", "text");
    title.setAttribute("x", "110");
    title.setAttribute("y", "120");
    title.setAttribute("text-anchor", "middle");
    title.setAttribute("class", "tower-cache-label");
    title.textContent = "Drobo";
    svg.append(title);
  }
  shape("circle", { cx: "43", cy: "230", r: "4", class: fresh ? "tower-light active" : "tower-light" });
  return svg;
}

function unitPortrait(pool: PoolStatus, fresh: boolean): HTMLElement {
  const portrait = element("div", "unit-portrait");
  const fallback = deviceArtwork(pool, fresh);
  portrait.append(fallback);
  if (isFiveD(pool)) {
    const image = document.createElement("img");
    image.src = "/local-artwork/Drobo5DFront.png";
    image.alt = "Drobo 5D enclosure";
    image.hidden = true;
    image.addEventListener("load", () => { fallback.style.display = "none"; image.hidden = false; }, { once: true });
    image.addEventListener("error", () => image.remove(), { once: true });
    portrait.append(image);
  }
  return portrait;
}

function capacityHistoryPanel(): HTMLElement {
  const panel = element("section", "statistics-dashboard");
  if (!capacityHistory && !capacityHistoryLoading && !capacityHistoryError) window.queueMicrotask(() => void loadCapacityHistory());
  if (capacityHistoryLoading && !capacityHistory) return liveStatus("Reading observations", "Loading recorded capacity measurements.", "quiet");
  if (capacityHistoryError) return liveStatus("History unavailable", capacityHistoryError, "error");
  if (!capacityHistory?.pools.length) return stateCard("Collecting history", "Statistics appear as OpenDrobo records actual device readings.", "quiet");
  const historyPool = capacityHistory.pools.find((pool) => pool.id === statisticsPoolId)
    ?? capacityHistory.pools.find((pool) => pool.id === selectedPoolId) ?? capacityHistory.pools[0];
  statisticsPoolId = historyPool.id;
  panel.append(textElement("h3", `${historyPool.model} · recorded observations`, "card-heading"));
  if (capacityHistory.pools.length > 1) {
    const label = textElement("label", "Recorded device", "field-label");
    const picker = document.createElement("select");
    picker.id = "statistics-device";
    picker.className = "device-select";
    for (const pool of capacityHistory.pools) picker.append(new Option(`${pool.model} · ${pool.id.slice(-6)}`, pool.id, false, pool.id === historyPool.id));
    picker.addEventListener("change", () => { statisticsPoolId = picker.value; render(); });
    label.append(picker);
    panel.append(label);
  }
  const ranges: Array<[StatisticsRange, string]> = [["1h", "1 hour"], ["6h", "6 hours"], ["24h", "24 hours"], ["7d", "7 days"], ["all", "All recorded"]];
  panel.append(segmentedControl("Statistics time range", ranges.map(([key, label]): [string, boolean, () => void] => [label, statisticsRange === key, () => { statisticsRange = key; render(); }])));
  const summary = analyzeCapacity(historyPool.samples, statisticsRange, Date.now());
  if (!summary.last || !summary.first) {
    panel.append(stateCard("No readings in this range", "Choose a wider time range to see retained observations.", "quiet"));
    return panel;
  }
  const last = summary.last;
  const cards = element("div", "statistics-metrics");
  cards.append(statisticCard("Protected used", formatDecimalBytes(last.usedBytes), `${((summary.utilizationBasisPoints ?? 0) / 100).toFixed(2)}% utilized`),
    statisticCard("Protected free", formatDecimalBytes(last.freeBytes), `Usable total ${formatDecimalBytes(last.totalBytes)}`),
    statisticCard("OS filesystem used", formatDecimalBytes(last.osUsedBytes), "Separate filesystem accounting"),
    statisticCard("Used-space change", signedBytes(summary.usedDeltaBytes), "Across the selected observations"),
    statisticCard("Observed average change", summary.averageDailyChangeBytes === null ? "Insufficient history" : `${signedBytes(summary.averageDailyChangeBytes)} / day`, summary.capacityChanged ? "Capacity changed during this range" : "Allocation change; not transfer speed or a forecast"),
    statisticCard("Recording coverage", `${summary.sampleCount} readings`, `${formatDuration(summary.spanMs)} observed span`));
  panel.append(cards);
  if (summary.sampleCount < 2) panel.append(stateCard("Collecting trend data", "Current values are available. Charts need two distinct readings.", "quiet"));
  else {
    const charts = element("div", "statistics-charts");
    const space = element("section", "statistics-chart-card glass");
    const metrics: Array<[typeof statisticsMetric, string]> = [["used", "Protected used"], ["free", "Protected free"], ["os", "OS used"]];
    space.append(textElement("h3", "Space over time", "card-heading"), segmentedControl("Space chart metric", metrics.map(([key, label]): [string, boolean, () => void] => [label, statisticsMetric === key, () => { statisticsMetric = key; render(); }])), statisticsChart(summary, statisticsMetric));
    const change = element("section", "statistics-chart-card glass");
    change.append(textElement("h3", "Change in protected usage", "card-heading"), textElement("p", "Difference from the first reading in this range.", "status-copy"), statisticsChart(summary, "change"));
    const utilization = element("section", "statistics-chart-card glass");
    utilization.append(textElement("h3", "Pool utilization", "card-heading"), textElement("p", "Protected used space as a share of usable capacity.", "status-copy"), statisticsChart(summary, "utilization"));
    charts.append(space, change, utilization);
    panel.append(charts);
  }
  const quality = element("section", "statistics-quality glass");
  const facts = element("dl", "capability-list");
  appendDefinition(facts, "First in range", formatHistoryTime(summary.first.timestampMs));
  appendDefinition(facts, "Last in range", formatHistoryTime(summary.last.timestampMs));
  appendDefinition(facts, "Used-space range", `${formatDecimalBytes(summary.minUsedBytes ?? last.usedBytes)} – ${formatDecimalBytes(summary.maxUsedBytes ?? last.usedBytes)}`);
  appendDefinition(facts, "OS-used change", signedBytes(summary.osUsedDeltaBytes));
  appendDefinition(facts, "Free-space change", signedBytes(summary.freeDeltaBytes));
  appendDefinition(facts, "Recording gaps >15 min", String(summary.gapCount));
  appendDefinition(facts, "Largest reading interval", summary.sampleCount > 1 ? formatDuration(summary.largestGapMs) : "Insufficient history");
  appendDefinition(facts, "Storage", capacityHistory.persistence === "disk" ? "Saved on this Mac" : "Memory only; persistence unavailable");
  quality.append(textElement("h3", "Observation quality", "card-heading"), facts, textElement("p", "Gaps mean no saved observation. They do not establish a device outage. Readings are retained in five-minute intervals, up to 2,048 globally; charts never fill missing periods.", "status-copy"));
  if (summary.rejectedCount) quality.append(textElement("p", `${summary.rejectedCount} invalid readings were excluded.`, "status-copy"));
  panel.append(quality, statisticsObservations(summary.samples));
  const current = poolStatus?.pools.find((pool) => pool.id === historyPool.id);
  if (current) panel.append(driveStatistics(current));
  panel.append(statisticsActivity(historyPool.id));
  return panel;
}

function statisticCard(label: string, value: string, description: string): HTMLElement {
  const card = element("article", "statistic-card glass");
  card.append(textElement("span", label), textElement("strong", value), textElement("p", description, "status-copy"));
  return card;
}

function signedBytes(raw: string | null): string {
  if (raw === null) return "Insufficient history";
  const value = BigInt(raw);
  return `${value > 0n ? "+" : value < 0n ? "−" : ""}${formatDecimalBytes((value < 0n ? -value : value).toString())}`;
}

function formatDuration(milliseconds: number): string {
  if (milliseconds < 60_000) return `${Math.floor(milliseconds / 1000)} sec`;
  if (milliseconds < 3_600_000) return `${Math.floor(milliseconds / 60_000)} min`;
  const hours = Math.floor(milliseconds / 3_600_000);
  return hours < 24 ? `${hours} hr ${Math.floor(milliseconds % 3_600_000 / 60_000)} min` : `${Math.floor(hours / 24)} days ${hours % 24} hr`;
}

function statisticsChart(summary: CapacityStatistics, metric: "used" | "free" | "os" | "change" | "utilization"): SVGSVGElement {
  const width = 560, height = 250, left = 90, right = 20, top = 20, bottom = 52;
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("viewBox", `0 0 ${width} ${height}`);
  svg.setAttribute("class", `statistics-chart metric-${metric}`);
  svg.setAttribute("role", "img");
  const first = summary.first!;
  const last = summary.last!;
  const value = (sample: CapacityObservation): bigint => metric === "utilization" ? BigInt(sample.usedBytes) * 10_000n / BigInt(sample.totalBytes)
    : metric === "change" ? BigInt(sample.usedBytes) - BigInt(first.usedBytes)
      : BigInt(metric === "free" ? sample.freeBytes : metric === "os" ? sample.osUsedBytes : sample.usedBytes);
  const values = summary.samples.map(value);
  const observedMaximum = summary.samples.reduce((maximum, sample) => BigInt(sample.totalBytes) > maximum ? BigInt(sample.totalBytes) : maximum, 1n);
  let minimum = 0n;
  let maximum = metric === "utilization" ? 10_000n : values.reduce((maximum, current) => current > maximum ? current : maximum, observedMaximum);
  if (metric === "change") {
    const extent = values.reduce((maximum, current) => (current < 0n ? -current : current) > maximum ? (current < 0n ? -current : current) : maximum, 1n);
    minimum = -extent;
    maximum = extent;
  }
  const format = (raw: bigint) => metric === "utilization" ? `${(Number(raw) / 100).toFixed(1)}%` : metric === "change" ? signedBytes(raw.toString()) : formatDecimalBytes(raw.toString());
  const duration = Math.max(1, last.timestampMs - first.timestampMs);
  const x = (sample: CapacityObservation) => left + (sample.timestampMs - first.timestampMs) / duration * (width - left - right);
  const y = (raw: bigint) => height - bottom - Number((raw - minimum) * 1_000_000n / (maximum - minimum)) / 1_000_000 * (height - top - bottom);
  svg.setAttribute("aria-label", `${metric} across ${summary.sampleCount} real readings, ${formatHistoryTime(first.timestampMs)} to ${formatHistoryTime(last.timestampMs)}. First ${format(value(first))}; last ${format(value(last))}. Recording gaps longer than fifteen minutes are not connected.`);
  for (const tick of [minimum, (minimum + maximum) / 2n, maximum]) {
    const line = document.createElementNS("http://www.w3.org/2000/svg", "line");
    line.setAttribute("x1", String(left)); line.setAttribute("x2", String(width - right));
    line.setAttribute("y1", String(y(tick))); line.setAttribute("y2", String(y(tick))); line.setAttribute("class", "history-axis statistics-gridline");
    svg.append(line, historyAxisText(format(tick), left - 9, y(tick) + 4, "end"));
  }
  for (const segment of summary.segments) {
    if (segment.samples.length > 1) {
      const line = document.createElementNS("http://www.w3.org/2000/svg", "polyline");
      line.setAttribute("points", segment.samples.map((sample) => `${x(sample).toFixed(2)},${y(value(sample)).toFixed(2)}`).join(" "));
      line.setAttribute("class", "history-line");
      svg.append(line);
    }
    const stride = Math.max(1, Math.ceil(summary.sampleCount / 80));
    segment.samples.forEach((sample, index) => {
      if (index % stride && index !== segment.samples.length - 1) return;
      const point = document.createElementNS("http://www.w3.org/2000/svg", "circle");
      point.setAttribute("cx", String(x(sample))); point.setAttribute("cy", String(y(value(sample)))); point.setAttribute("r", "3"); point.setAttribute("class", "statistics-point");
      const title = document.createElementNS("http://www.w3.org/2000/svg", "title");
      title.textContent = `${formatHistoryTime(sample.timestampMs)} · ${format(value(sample))}`;
      point.append(title); svg.append(point);
    });
  }
  svg.append(historyAxisText(formatHistoryDate(first.timestampMs), left, height - 15, "start"), historyAxisText(formatHistoryDate(last.timestampMs), width - right, height - 15, "end"));
  return svg;
}

function statisticsObservations(samples: readonly CapacityObservation[]): HTMLElement {
  const details = element("details", "advanced-details");
  details.append(textElement("summary", "Recorded observations · newest 50"));
  const wrap = element("div", "statistics-table-scroll");
  const table = element("table", "storage-table");
  table.append(tableHeader(["Time", "Protected used", "Protected free", "OS used"]));
  const body = element("tbody");
  for (const sample of [...samples].reverse().slice(0, 50)) {
    const row = element("tr");
    for (const value of [formatHistoryTime(sample.timestampMs), formatDecimalBytes(sample.usedBytes), formatDecimalBytes(sample.freeBytes), formatDecimalBytes(sample.osUsedBytes)]) appendTableCell(row, value);
    body.append(row);
  }
  table.append(body); wrap.append(table); details.append(wrap);
  return details;
}

function driveStatistics(pool: PoolStatus): HTMLElement {
  const panel = element("section", "statistics-drive-card glass");
  panel.append(textElement("h3", isFiveD(pool) ? "Installed data drives" : "Reported physical slots", "card-heading"));
  const slots = dataSlots(pool);
  const total = slots.reduce((sum, slot) => sum + safeBytes(slot.physicalCapacityBytes), 0n);
  const measuredSlots = slots.filter((slot) => safeBytes(slot.physicalCapacityBytes) > 0n).length;
  panel.append(textElement("p", `${total > 0n ? `${formatDecimalBytes(total.toString())} reported across ${measuredSlots} slots.` : "Physical capacity is unavailable in this snapshot."} Per-drive used space is not reported.${poolIsStale() ? " Last snapshot is stale." : ""}`, "status-copy"));
  for (const slot of slots) {
    const row = element("div", "statistics-drive-row");
    const bar = document.createElement("meter");
    bar.min = 0; bar.max = 1; bar.value = total > 0n ? Number(safeBytes(slot.physicalCapacityBytes) * 10_000n / total) / 10_000 : 0;
    bar.setAttribute("aria-label", `Slot ${slot.slot} share of reported physical capacity`);
    const measured = safeBytes(slot.physicalCapacityBytes) > 0n;
    row.append(textElement("strong", isFiveD(pool) ? `Drive ${slot.slot + 1}` : `Slot ${slot.slot}`), measured ? bar : textElement("span", "Not reported"), textElement("span", measured ? formatPhysicalCapacity(slot.physicalCapacityBytes) : "Unavailable"), textElement("small", slot.manufacturerModel ?? "Make / model unavailable"));
    panel.append(row);
  }
  const cache = cacheSlot(pool);
  if (cache) {
    const row = element("section", "statistics-ssd");
    row.append(textElement("h4", "mSATA cache SSD", "card-heading"), textElement("p", `${cache.manufacturerModel ?? "SSD"} · ${formatPhysicalCapacity(cache.physicalCapacityBytes)} · cache usage unavailable`, "status-copy"));
    if (cache.ssdLifeRemainingPercent !== null) {
      const progress = document.createElement("progress");
      progress.max = 100; progress.value = cache.ssdLifeRemainingPercent;
      progress.setAttribute("aria-label", "Reported SSD life remaining");
      row.append(textElement("strong", `${cache.ssdLifeRemainingPercent}% reported life remaining`), progress);
    } else row.append(textElement("p", "SSD life not reported", "status-copy"));
    panel.append(row);
  }
  return panel;
}

function statisticsActivity(unitId: string): HTMLElement {
  const panel = element("section", "statistics-quality glass");
  panel.append(textElement("h3", "Retained session events", "card-heading"));
  if (eventsError || !eventsResponse) {
    panel.append(textElement("p", eventsError ? "Session events unavailable." : "Reading session observations…", "status-copy"));
    return panel;
  }
  const retained = eventsResponse.events.filter((event) => event.unitId === unitId);
  const counts = element("div", "statistics-event-counts");
  for (const [label, count] of [["Device events", retained.length], ["Capacity changes", retained.filter((event) => event.kind === "protected-capacity-changed").length], ["Slot-state changes", retained.filter((event) => event.kind === "numeric-slot-status-changed").length], ["SSD-life changes", retained.filter((event) => event.kind === "ssd-wear-changed").length]] as const) counts.append(statisticCard(label, String(count), "Current application session"));
  panel.append(counts, textElement("p", `Latest bounded observations since ${eventsResponse.observedSinceMs ? formatHistoryTime(eventsResponse.observedSinceMs) : "session start"}. These are OpenDrobo observations, not recovered onboard logs.`, "status-copy"));
  return panel;
}

function historyAxisText(value: string, x: number, y: number, anchor: "start" | "end"): SVGTextElement {
  const text = document.createElementNS("http://www.w3.org/2000/svg", "text");
  text.setAttribute("x", String(x)); text.setAttribute("y", String(y)); text.setAttribute("text-anchor", anchor); text.setAttribute("class", "history-label"); text.textContent = value;
  return text;
}

function formatHistoryDate(timestampMs: number): string {
  return new Date(timestampMs).toLocaleString(undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
}

function formatHistoryTime(timestampMs: number): string {
  return new Date(timestampMs).toLocaleString();
}

function byteRatio(part: string, total: string): number {
  try {
    const whole = BigInt(total);
    if (whole <= 0n) return 0;
    return Number((BigInt(part) * 10_000n) / whole) / 10_000;
  } catch {
    return 0;
  }
}

function poolLunTable(pool: PoolStatus): HTMLElement {
  const block = element("div", "pool-detail");
  block.append(textElement("h4", "Logical LUN detail", "card-heading"));
  const table = element("table", "storage-table pool-table");
  table.append(tableHeader(["LUN", "Logical total (IEC)", "OS filesystem used (IEC)"]));
  const body = document.createElement("tbody");
  for (const volume of pool.volumes) {
    const row = document.createElement("tr");
    appendTableCell(row, String(volume.lun));
    appendTableCell(row, formatBytes(volume.logicalTotalBytes));
    appendTableCell(row, formatBytes(volume.osUsedBytes));
    body.append(row);
  }
  table.append(body);
  block.append(table, textElement("p", "Logical LUN values are OS filesystem detail. They do not override Finder capacity reporting.", "status-copy"));
  return block;
}

function poolSlotTable(pool: PoolStatus): HTMLElement {
  const block = element("div", "pool-detail");
  block.append(textElement("h4", "Detected physical slots", "card-heading"));
  if (pool.slots.length === 0) {
    block.append(textElement("p", "No physical slot detail was reported by the original service.", "status-copy"));
    return block;
  }
  const table = element("table", "storage-table pool-table");
  table.append(tableHeader(["Slot", "Kind", "Physical capacity", "SSD life", "Cache used"]));
  const body = document.createElement("tbody");
  for (const slot of pool.slots) {
    const row = document.createElement("tr");
    appendTableCell(row, String(slot.slot));
    appendTableCell(row, slot.kind === "ssd" ? "Cache SSD" : "Data drive");
    appendTableCell(row, formatPhysicalCapacity(slot.physicalCapacityBytes));
    appendTableCell(row, slot.ssdLifeRemainingPercent === null ? "Not reported" : `${slot.ssdLifeRemainingPercent}% remaining`);
    appendTableCell(row, "Unknown (not reported)");
    body.append(row);
  }
  table.append(body);
  block.append(table, textElement("p", "Slot presence is detected only. OpenDrobo does not infer mapped drive health or cache usage from raw status.", "status-copy"));
  return block;
}

function volumesPanel(content: HTMLElement): HTMLElement {
  const intro = panelIntro("Volumes", "Drobo volumes only. Used space is the operating-system filesystem reading, separate from protected pool capacity.");
  const refresh = actionButton(storageLoading ? "Refreshing…" : "Refresh volumes", "primary", () => void loadStorageStatus());
  refresh.disabled = storageLoading;
  const actions = element("div", "panel-actions");
  actions.append(refresh);
  intro.append(actions);
  content.append(intro);

  if (!storageStatus && !storageLoading && !storageError) window.queueMicrotask(() => void loadStorageStatus());
  if (storageLoading && !storageStatus) {
    content.append(liveStatus("Reading mounted volumes", "OpenDrobo is requesting local, read-only OS volume metadata.", "quiet"));
  } else if (storageError) {
    content.append(liveStatus("Volume metadata unavailable", storageError, "error"));
  } else if (!storageStatus) {
    content.append(stateCard("Desktop runtime required", "Open the desktop application to inspect local mounted-volume metadata.", "quiet"));
  } else if (storageStatus.volumes.filter((volume) => volume.provider === "drobo").length === 0) {
    content.append(stateCard("No Drobo volumes reported", storageStatus.message || "No mounted Drobo volumes were returned by the local service.", "quiet"));
  } else {
    const volumes = storageStatus.volumes.filter((volume) => volume.provider === "drobo");
    const card = element("section", "storage-table-wrap glass");
    const table = element("table", "storage-table");
    const caption = textElement("caption", "Drobo volumes");
    table.append(caption, tableHeader(["Name", "Filesystem used"]));
    const body = document.createElement("tbody");
    for (const volume of volumes) {
      const row = document.createElement("tr");
      appendTableCell(row, volume.name || volume.id, "volume-name");
      appendTableCell(row, formatNullableBytes(volume.usedBytes));
      body.append(row);
    }
    table.append(body);
    const advanced = document.createElement("details");
    advanced.className = "advanced-details";
    advanced.append(textElement("summary", "Advanced volume details"), droboVolumeTechnicalTable(volumes));
    card.append(table, textElement("p", storageStatus.message, "status-copy"), advanced);
    content.append(card);
  }
  content.append(textElement("p", "This view never mounts, formats, repairs, or changes a volume.", "scope-note"));
  return content;
}

function droboVolumeTechnicalTable(volumes: StorageVolume[]): HTMLElement {
  const table = element("table", "storage-table");
  table.append(tableHeader(["Filesystem", "Mount path", "Logical total", "Available", "Access"]));
  const body = document.createElement("tbody");
  for (const volume of volumes) {
    const row = document.createElement("tr");
    appendTableCell(row, volume.filesystem || "Unknown");
    appendTableCell(row, volume.mountPath ?? "Not mounted", "path-cell");
    appendTableCell(row, formatNullableBytes(volume.totalBytes));
    appendTableCell(row, formatNullableBytes(volume.availableBytes));
    appendTableCell(row, volume.readOnly ? "Read-only" : "Read/write");
    body.append(row);
  }
  table.append(body);
  return table;
}

function consolePanel(content: HTMLElement): HTMLElement {
  const intro = panelIntro("Console", "OpenDrobo events only: bounded, sanitized local application and device-operation records. This is not a system-wide or kernel log viewer.");
  const actions = element("div", "panel-actions");
  const refresh = actionButton(consoleLoading ? "Refreshing…" : "Refresh", "primary", () => void loadConsoleRecords(true));
  refresh.id = "console-refresh";
  refresh.disabled = consoleLoading;
  const pause = actionButton(consolePaused ? "Resume polling" : "Pause polling", "secondary", () => {
    consolePaused = !consolePaused;
    render();
  });
  pause.id = "console-pause";
  actions.append(refresh, pause);
  intro.append(actions);
  content.append(intro);

  if (!consoleResponse && !consoleLoading && !consoleError) window.queueMicrotask(() => void loadConsoleRecords(true));
  if (consoleLoading && !consoleResponse) content.append(stateCard("Loading OpenDrobo events", "Reading up to 200 bounded, sanitized local records.", "quiet"));
  if (consoleError) content.append(stateCard("Console unavailable", consoleError, "error"));
  if (!consoleResponse) return content;

  const toolbar = element("div", "console-toolbar glass");
  const searchLabel = textElement("label", "Search events", "field-label");
  searchLabel.setAttribute("for", "console-search");
  const search = document.createElement("input");
  search.id = "console-search";
  search.className = "console-search";
  search.type = "search";
  search.placeholder = "Source, operation, or message";
  search.value = consoleSearch;
  search.addEventListener("input", () => {
    consoleSearch = search.value;
    render();
  });
  searchLabel.append(search);
  const levelLabel = textElement("label", "Level", "field-label");
  levelLabel.setAttribute("for", "console-level");
  const level = document.createElement("select");
  level.id = "console-level";
  level.className = "device-select";
  for (const optionValue of ["all", "info", "warning", "error"] as ConsoleLevelFilter[]) {
    const option = document.createElement("option");
    option.value = optionValue;
    option.textContent = optionValue === "all" ? "All levels" : optionValue[0].toUpperCase() + optionValue.slice(1);
    option.selected = consoleLevelFilter === optionValue;
    level.append(option);
  }
  level.addEventListener("change", () => {
    consoleLevelFilter = level.value as ConsoleLevelFilter;
    render();
  });
  levelLabel.append(level);
  const visibleRecords = filteredConsoleRecords();
  const copy = actionButton("Copy visible", "secondary", () => void copyConsoleRecords(visibleRecords));
  copy.id = "console-copy";
  copy.disabled = visibleRecords.length === 0;
  toolbar.append(searchLabel, levelLabel, copy);
  content.append(toolbar);
  if (consoleClipboardStatus) content.append(textElement("p", consoleClipboardStatus, "status-copy"));

  const events = element("section", "console-events glass");
  events.id = "console-events";
  events.tabIndex = 0;
  events.setAttribute("role", "region");
  events.setAttribute("aria-label", "OpenDrobo events");
  events.setAttribute("aria-live", "off");
  if (visibleRecords.length === 0) {
    events.append(textElement("p", "No OpenDrobo events match the current filters.", "status-copy"));
  } else {
    for (const record of visibleRecords) events.append(consoleRecord(record));
  }
  content.append(events, textElement("p", `${consoleResponse.message} Persistence: ${consoleResponse.persistence}. ${consolePaused ? "Polling paused." : "Polling every 2 seconds while this Console section is visible."}`, "scope-note"));
  return content;
}

function sharingPanel(content: HTMLElement): HTMLElement {
  const intro = panelIntro("Sharing", "Inspect existing local SMB/NFS service status and SMB exports. Existing HFS+ files can be shared with clients through SMB or NFS; no reformat is required.");
  const actions = element("div", "panel-actions");
  const refresh = actionButton(sharingLoading ? "Refreshing…" : "Refresh sharing", "primary", () => void loadSharingStatus());
  refresh.disabled = sharingLoading;
  actions.append(refresh);
  intro.append(actions);
  content.append(intro);

  if (!sharingStatus && !sharingLoading && !sharingError) window.queueMicrotask(() => void loadSharingStatus());
  if (sharingLoading && !sharingStatus) content.append(liveStatus("Inspecting sharing", "Checking existing local SMB/NFS status and SMB exports only.", "quiet"));
  if (sharingError) content.append(liveStatus("Sharing inspection unavailable", sharingError, "error"));
  if (sharingStatus) {
    const services = element("section", "sharing-grid");
    if (sharingStatus.services.length === 0) {
      services.append(stateCard("No sharing services reported", sharingStatus.message || "The local system returned no SMB or NFS service information.", "quiet"));
    } else {
      for (const service of sharingStatus.services) {
        const card = element("article", "share-service glass");
        card.append(textElement("span", service.state, `service-state ${service.state}`), textElement("h3", service.name), textElement("p", service.supported ? service.message : `Unsupported: ${service.message}`, "status-copy"));
        services.append(card);
      }
    }
    content.append(services);
    const exportsCard = element("section", "storage-table-wrap glass");
    if (!sharingStatus.exportsAvailable) {
      exportsCard.append(textElement("h3", "SMB export inspection", "card-heading"), textElement("p", "SMB export inspection is unavailable or unknown in this local runtime. NFS is service-status only. OpenDrobo does not enable sharing or change access policy.", "status-copy"));
    } else if (sharingStatus.exports.length === 0) {
      exportsCard.append(textElement("h3", "Existing SMB exports", "card-heading"), textElement("p", "No configured SMB exports were reported. NFS is service-status only. OpenDrobo does not enable sharing or change access policy.", "status-copy"));
    } else {
      const table = element("table", "storage-table");
      table.append(textElement("caption", "Existing SMB exports — inspect only"), tableHeader(["Name", "Path", "Protocol", "Access", "Guest"]));
      const body = document.createElement("tbody");
      for (const share of sharingStatus.exports) {
        const row = document.createElement("tr");
        appendTableCell(row, share.name);
        appendTableCell(row, share.path, "path-cell");
        appendTableCell(row, share.protocol);
        appendTableCell(row, nullableBooleanLabel(share.readOnly, "Read-only", "Read/write"));
        appendTableCell(row, nullableBooleanLabel(share.guestAllowed, "Allowed", "Not allowed"));
        body.append(row);
      }
      table.append(body);
      exportsCard.append(table);
    }
    exportsCard.append(textElement("p", sharingStatus.message, "status-copy"));
    content.append(exportsCard);
    content.append(sharingConfigurationCard(sharingStatus));
  } else if (!sharingLoading && !sharingError) {
    content.append(stateCard("Desktop runtime required", "Open the desktop application to inspect existing local sharing configuration.", "quiet"));
  }
  return content;
}

function sharingConfigurationCard(status: SharingStatusResponse): HTMLElement {
  const card = element("section", "sharing-config glass");
  card.append(textElement("h3", "Native permissions and configuration", "card-heading"), textElement("p", `${status.configuration.message} Folder ownership and ACLs remain native operating-system authority. Share-level read-only settings do not replace folder rights.`, "status-copy"));
  const macos = status.platform.toLocaleLowerCase().includes("mac");
  if (macos && status.configuration.settingsActionAvailable) {
    card.append(actionButton("Configure sharing in macOS", "secondary", () => void openSharingSettings()));
  } else if (macos) {
    card.append(textElement("p", "Native File Sharing settings are not available from this desktop runtime.", "status-copy"));
  } else {
    card.append(textElement("p", "Windows and Linux native sharing adapters are planned or unavailable. OpenDrobo does not create accounts or change ACLs.", "status-copy"));
  }
  if (sharingSettingsStatus) card.append(textElement("p", sharingSettingsStatus, "status-copy"));
  return card;
}

function consoleRecord(record: ConsoleRecord): HTMLElement {
  const row = element("article", `console-record ${record.level}`);
  const meta = element("div", "console-record-meta");
  meta.append(textElement("time", formatConsoleTime(record.timestampMs)), textElement("span", record.level, `console-level ${record.level}`), textElement("span", record.source), textElement("span", record.operation));
  row.append(meta, textElement("p", record.message));
  return row;
}

function mcpCard(): HTMLElement {
  const card = element("section", "mcp-card glass");
  card.append(textElement("h3", "Local MCP access", "card-heading"), textElement("p", "Starts only with --mcp-stdio. No MCP network listener, device control, file reader, or shell is exposed.", "status-copy"), textElement("p", "Client may receive mounted-volume, share, device, capacity, SSD, event, and history metadata, then may forward it. Choose trusted client. OpenDrobo keeps bounded local logs/history writes.", "status-copy"));
  const command = appInfo?.mcpCommandPath ?? "/absolute/path/to/opendrobo-desktop";
  const config = JSON.stringify({ mcpServers: { opendrobo: { command, args: ["--mcp-stdio"] } } }, null, 2);
  const code = textElement("pre", config, "mcp-config");
  const copy = actionButton("Copy MCP config", "secondary", () => void copyText(config, "MCP configuration copied.", "mcp"));
  card.append(code, copy);
  if (!appInfo?.mcpCommandPath) card.append(textElement("p", "Replace the absolute command placeholder with this app bundle's executable path before using the configuration.", "status-copy"));
  card.append(textElement("p", "Advanced provenance: data depends on an unauthenticated original loopback service. It is not security-attested.", "scope-note"));
  if (mcpClipboardStatus) card.append(textElement("p", mcpClipboardStatus, "status-copy"));
  return card;
}

function tableHeader(labels: string[]): HTMLTableSectionElement {
  const head = document.createElement("thead");
  const row = document.createElement("tr");
  for (const label of labels) row.append(textElement("th", label));
  head.append(row);
  return head;
}

function appendTableCell(row: HTMLTableRowElement, value: string, className?: string): void {
  row.append(textElement("td", value, className));
}

function formatPhysicalCapacity(raw: string): string {
  return safeBytes(raw) > 0n ? formatDecimalBytes(raw) : "Not reported";
}

function formatNullableBytes(raw: string | null): string {
  return raw ? formatBytes(raw) : "Not reported";
}

function nullableBooleanLabel(value: boolean | null, trueLabel: string, falseLabel: string): string {
  if (value === null) return "Not reported";
  return value ? trueLabel : falseLabel;
}

function formatConsoleTime(timestampMs: number): string {
  const date = new Date(timestampMs);
  return Number.isNaN(date.getTime()) ? "Unknown time" : date.toLocaleString();
}

function observationStateCopy(state: ObservationState): string {
  if (state === "active") return "Active — present in a fresh original-service snapshot";
  if (state === "inactive") return "Inactive — not present in the latest original-service snapshot";
  if (state === "stale") return "Stale — the last observation has expired";
  if (state === "read-error") return "Read error — the observation engine could not read this state";
  return "Unknown — source did not report a current presence state";
}

function formatRawDeviceStatus(status: PoolStatus["deviceStatus"]): string {
  return `mStatus=${status.mStatus}, mStatusEx=${status.mStatusEx}, mDiskPackStatus=${status.mDiskPackStatus}`;
}

function formatRawSlotStatus(status: PoolSlot["rawStatus"]): string {
  return `mStatus=${status.mStatus}, mDiskState=${status.mDiskState}, mDiskType=${status.mDiskType}`;
}

function filteredConsoleRecords(): ConsoleRecord[] {
  const query = consoleSearch.trim().toLocaleLowerCase();
  return [...(consoleResponse?.records ?? [])]
    .sort((a, b) => b.timestampMs - a.timestampMs || b.id.localeCompare(a.id))
    .filter((record) => {
      if (consoleLevelFilter !== "all" && record.level !== consoleLevelFilter) return false;
      if (!query) return true;
      return [record.source, record.operation, record.message, record.level].some((value) => value.toLocaleLowerCase().includes(query));
    });
}

function filteredDeviceEvents(): DeviceEvent[] {
  const query = eventsSearch.trim().toLocaleLowerCase();
  return [...(eventsResponse?.events ?? [])]
    .sort((a, b) => b.timestampMs - a.timestampMs || b.id.localeCompare(a.id))
    .filter((record) => {
      if (eventsSeverityFilter !== "all" && record.severity !== eventsSeverityFilter) return false;
      if (!query) return true;
      return [record.source, record.unitId ?? "", record.message, record.kind ?? "", record.slot === null || record.slot === undefined ? "" : String(record.slot)].some((value) => value.toLocaleLowerCase().includes(query));
    });
}

function deviceCard(device: Device): HTMLElement {
  const card = element("article", "device-card glass");
  card.classList.toggle("device-selected", selectedDeviceId === device.id);
  card.append(textElement("div", "▦", "device-glyph"), textElement("h2", device.name));
  const detail = element("dl", "device-detail");
  appendDefinition(detail, "Connection", device.connection);
  appendDefinition(detail, "Driver", device.driverReady ? "Ready" : "Not ready");
  card.append(detail);
  const select = actionButton(selectedDeviceId === device.id ? "Selected" : "Select", "secondary", () => {
    selectedDeviceId = device.id;
    capacity = null;
    capacityRequestedAtMs = null;
    render();
  });
  select.setAttribute("aria-label", `Select ${device.name}`);
  card.append(select);
  return card;
}

function capacityPanel(content: HTMLElement): HTMLElement {
  const intro = panelIntro("Capacity", "Usable capacity and installed drives.");
  const actions = element("div", "panel-actions");
  const readPool = actionButton(poolLoading ? "Refreshing…" : "Refresh capacity", "primary", () => void loadPoolStatus(true));
  readPool.disabled = poolLoading;
  actions.append(readPool, segmentedControl("Capacity view", [["Capacity", capacityMode === "capacity", () => { capacityMode = "capacity"; render(); }], ["Usage", capacityMode === "usage", () => { capacityMode = "usage"; render(); }]]));
  intro.append(actions);
  content.append(intro);
  const pools = poolStatus?.pools ?? [];
  if (pools.length) content.append(poolSelector("capacity-pool"));
  const pool = currentPool();
  if (pool) {
    const stale = poolIsStale();
    if (stale) content.append(liveStatus("Pool snapshot stale", `Last reported ${new Date(poolStatus?.observedAtMs ?? 0).toLocaleString()}. Values below are last reported, not current.`, "quiet"));
    content.append(capacityWorkspace(pool, stale));
  } else content.append(stateCard(pools.length ? "Choose a Drobo" : "Capacity unavailable", pools.length ? "Select a Drobo to view usable capacity." : "Device information has not been reported yet.", "quiet"));
  const devices = inventory?.devices ?? [];
  if (devices.length === 0) {
    content.append(stateCard("No protected-capacity endpoint selected", "The shared-pool diagnostic view works without registry endpoints. Guarded protected-capacity reads require a selected registry endpoint.", "quiet"));
    return content;
  }
  if (!selectedDeviceId || !devices.some((device) => device.id === selectedDeviceId)) selectedDeviceId = devices[0].id;
  const selectLabel = textElement("label", "Selected Drobo", "field-label");
  selectLabel.setAttribute("for", "capacity-device");
  const select = document.createElement("select");
  select.id = "capacity-device";
  select.className = "device-select";
  for (const device of devices) {
    const option = document.createElement("option");
    option.value = device.id;
    option.textContent = device.name;
    option.selected = device.id === selectedDeviceId;
    select.append(option);
  }
  select.addEventListener("change", () => {
    selectedDeviceId = select.value;
    capacity = null;
    capacityRequestedAtMs = null;
    capacityRequestSequence += 1;
    capacityLoading = false;
    clearCapacityExpiry();
    requestGeneration += 1;
    render();
  });
  const selector = element("div", "capacity-selector glass");
  selector.append(selectLabel, select);
  const read = actionButton(capacityLoading ? "Reading…" : "Read capacity", "primary", () => void readCapacity());
  read.disabled = capacityLoading;
  selector.append(read);
  const advanced = document.createElement("details");
  advanced.className = "advanced-details";
  advanced.append(textElement("summary", "Advanced protected-capacity verification"), selector, capacityCard());
  content.append(advanced);
  return content;
}

function capacityWorkspace(pool: PoolStatus, stale: boolean): HTMLElement {
  const workspace = element("section", `capacity-workspace glass${stale ? " stale-values" : ""}`);
  const installed = dataSlots(pool).reduce((sum, slot) => sum + safeBytes(slot.physicalCapacityBytes), 0n).toString();
  if (capacityMode === "usage") {
    const usage = element("div", "capacity-right");
    usage.append(textElement("h3", stale ? "Last reported installed physical drive capacity" : "Installed physical drive capacity"), textElement("strong", formatDecimalBytes(installed), "usage-total"), textElement("p", "Installed capacity is separate from protected pool capacity. Used/free/protection breakdown: Unavailable.", "status-copy"));
    const cache = element("section", "cache-summary");
    const ssd = cacheSlot(pool);
    cache.append(textElement("h3", "SSD cache", "card-heading"), textElement("p", ssd ? `Capacity: ${formatPhysicalCapacity(ssd.physicalCapacityBytes)}. Life: ${ssd.ssdLifeRemainingPercent === null ? "Not reported" : `${ssd.ssdLifeRemainingPercent}% remaining`}. Usage: Unavailable.` : "No SSD cache reported. Usage: Unavailable.", "status-copy"));
    workspace.append(usage, cache); return workspace;
  }
  const total = pool.protectedTotalBytes;
  const used = pool.protectedUsedBytes;
  const free = pool.protectedFreeBytes;
  const donut = capacityDonutValues("Protected pool capacity", total, used, free);
  const legend = element("div", "capacity-legend");
  for (const [name, value] of [["Used", used], ["Free", free], ["Total", total]]) legend.append(textElement("span", `${name}: ${formatDecimalBytes(value)}`));
  const right = element("div", "capacity-right"); right.append(textElement("h3", stale ? "Last reported protected pool capacity" : "Protected pool capacity"), donut, legend, textElement("p", `OS filesystem used: ${formatDecimalBytes(pool.osUsedBytes)}. OS usage and protected pool allocation are separate measurements.`, "status-copy"));
  const cache = element("section", "cache-summary");
  const ssd = cacheSlot(pool);
  cache.append(textElement("h3", "SSD cache", "card-heading"), textElement("p", ssd ? `Capacity: ${formatPhysicalCapacity(ssd.physicalCapacityBytes)}. Life: ${ssd.ssdLifeRemainingPercent === null ? "Not reported" : `${ssd.ssdLifeRemainingPercent}% remaining`}. Usage: Unavailable.` : "No SSD cache reported. Usage: Unavailable.", "status-copy"));
  workspace.append(right, cache);
  return workspace;
}

function capacityDonutValues(label: string, total: string, used: string, free: string): HTMLElement {
  const block = element("section", "capacity-donut capacity-donut-large");
  const ratio = byteRatio(used, total); const circumference = 2 * Math.PI * 42;
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg"); svg.setAttribute("viewBox", "0 0 110 110"); svg.setAttribute("role", "img"); svg.setAttribute("aria-label", `${label}: used ${formatDecimalBytes(used)}, free ${formatDecimalBytes(free)}, total ${formatDecimalBytes(total)}.`);
  for (const [className, dash] of [["donut-free", null], ["donut-used", `${(circumference * ratio).toFixed(2)} ${circumference.toFixed(2)}`]] as const) { const circle = document.createElementNS("http://www.w3.org/2000/svg", "circle"); circle.setAttribute("cx", "55"); circle.setAttribute("cy", "55"); circle.setAttribute("r", "42"); circle.setAttribute("class", className); if (dash) circle.setAttribute("stroke-dasharray", dash); svg.append(circle); }
  const center = element("div", "donut-center");
  center.append(textElement("strong", formatDecimalBytes(total)), textElement("span", "Usable capacity"));
  block.append(svg, center); return block;
}

function capacityCard(): HTMLElement {
  const card = element("section", "capacity-card glass");
  const stale = capacityIsStale();
  if (!capacity) return stateCard("Capacity unknown", "Select device, then use Read capacity. No hardware read runs on launch.", "quiet");
  if (stale) return stateCard("Capacity expired", "Last read is older than 30 seconds. Read again for current protected-capacity data.", "quiet");
  if (capacity.state !== "available") return stateCard(capacity.state === "error" ? "Capacity read failed" : "Capacity unavailable", capacity.message, capacity.state === "error" ? "error" : "quiet");
  const values: Array<[string, string | null]> = [
    ["Protected total (IEC)", capacity.protectedTotalBytes],
    ["Protected used (IEC)", capacity.protectedUsedBytes],
    ["Protected free (IEC)", capacity.protectedFreeBytes]
  ];
  for (const [label, value] of values) {
    const stat = element("div", "capacity-stat");
    stat.append(textElement("span", label), textElement("strong", value ? formatBytes(value) : "Unknown"));
    card.append(stat);
  }
  const observed = capacity.observedAtMs ? new Date(capacity.observedAtMs).toLocaleTimeString() : "Unknown";
  card.append(textElement("p", `${capacity.message} Observed: ${observed}.`, "status-copy"));
  return card;
}

function preferencesPanel(content: HTMLElement): HTMLElement {
  content.append(panelIntro("Dashboard Preferences", "App release checks are optional. Firmware updates and automatic download are not available."));
  const settings = element("section", "preferences-card glass");
  const label = textElement("label", "Check OpenDrobo app releases automatically", "toggle-row");
  const toggle = document.createElement("input");
  toggle.type = "checkbox";
  toggle.checked = autoUpdateChecks;
  toggle.addEventListener("change", () => {
    autoUpdateChecks = toggle.checked;
    localStorage.setItem("opendrobo.autoUpdateChecks", String(autoUpdateChecks));
    if (autoUpdateChecks) void checkUpdates();
  });
  label.prepend(toggle);
  settings.append(label, textElement("p", "Checks OpenDrobo GitHub releases when enabled or manually requested. No automatic download or installation.", "status-copy"));
  const actions = element("div", "update-actions");
  const check = actionButton(updateLoading ? "Checking…" : "Check now", "primary", () => void checkUpdates());
  check.disabled = updateLoading;
  actions.append(check);
  if (updateResult?.state === "available" && updateResult.releaseUrl) {
    actions.append(actionButton("Open GitHub release", "secondary", () => void openRelease(updateResult?.releaseUrl ?? "")));
  }
  settings.append(actions);
  if (updateResult) settings.append(stateCard(updateTitle(updateResult.state), updateResult.message, updateResult.state === "error" ? "error" : "quiet"));
  content.append(settings);
  const discovery = element("section", "preferences-card glass");
  discovery.append(textElement("h3", "Discovery", "card-heading"), disabledCheckbox("Automatically discover Drobos", "Configuration unavailable"), disabledTextField("Manual device address"), textElement("p", "Local discovery remains available from All Drobos.", "status-copy"));
  const notifications = element("section", "preferences-card glass");
  notifications.append(textElement("h3", "Dashboard alerts", "card-heading"), disabledOptions("Alert threshold", ["Useful", "Important", "Critical"]), disabledCheckbox("Email alerts", "Unavailable"), unavailableSettingButton("Configure email"));
  const appearance = element("section", "preferences-card glass");
  appearance.append(textElement("h3", "Application", "card-heading"), disabledCheckbox("Show in system tray", "Unavailable"));
  const language = textElement("label", "Preferred language", "field-label");
  const select = document.createElement("select");
  select.disabled = true;
  select.className = "device-select";
  const english = document.createElement("option");
  english.textContent = "English";
  select.append(english);
  language.append(select);
  appearance.append(language, textElement("p", "Additional languages and notification preferences are not implemented yet.", "status-copy"));
  content.append(discovery, notifications, appearance);
  return content;
}

function statisticsPanel(content: HTMLElement): HTMLElement {
  const intro = panelIntro("Statistics", "Capacity readings recorded over time by OpenDrobo.");
  const refresh = actionButton(capacityHistoryLoading ? "Refreshing…" : "Refresh statistics", "primary", () => { void loadCapacityHistory(); void loadDeviceEvents(true); });
  refresh.disabled = capacityHistoryLoading; intro.append(element("div", "panel-actions")); (intro.lastElementChild as HTMLElement).append(refresh);
  content.append(intro, capacityHistoryPanel()); return content;
}

function toolsPanel(content: HTMLElement): HTMLElement {
  content.append(panelIntro("Tools", "Device operations and software updates."), controlStatusPanel());
  const updates = element("section", "tool-group glass");
  updates.append(textElement("h3", "Software Updates"), textElement("p", "App updates come from OpenDrobo releases. Device firmware updates are unavailable.", "status-copy"));
  const actions = element("div", "panel-actions");
  const check = actionButton(updateLoading ? "Checking…" : "Check app updates", "secondary", () => void checkUpdates());
  check.disabled = updateLoading;
  actions.append(check, unavailableSettingButton("Manual device update"));
  if (updateResult?.state === "available" && updateResult.releaseUrl) actions.append(actionButton("Open release", "secondary", () => void openRelease(updateResult?.releaseUrl ?? "")));
  updates.append(actions);
  if (updateResult) updates.append(liveStatus(updateTitle(updateResult.state), updateResult.message, updateResult.state === "error" ? "error" : "quiet"));
  const repair = element("section", "tool-group glass");
  repair.append(textElement("h3", "Reset / Repair"));
  const repairActions = element("div", "panel-actions");
  for (const name of ["Format", "Device Repair", "Reset"]) repairActions.append(unavailableSettingButton(name));
  repair.append(repairActions, textElement("p", "Formatting, repair and reset are unavailable. No disk changes can be submitted here.", "status-copy"));
  content.append(updates, repair);
  return content;
}

function settingsPanel(content: HTMLElement): HTMLElement {
  content.append(panelIntro("Drobo Settings", "Settings for the selected device. Current values stay unknown until read from the device."));
  if (poolStatus?.pools.length && (poolStatus.pools.length > 1 || selectedPoolId === null)) content.append(poolSelector("settings-pool"));
  const pool = currentPool();
  if (!pool) {
    content.append(stateCard("Select a Drobo", "Choose a device from All Drobos to see its applicable settings.", "quiet"));
    return content;
  }
  if (!isFiveD(pool)) {
    content.append(stateCard(`${pool.model} settings`, "A verified settings profile is not available for this model. Device information remains available under Status.", "quiet"));
    return content;
  }
  const layout = element("div", "device-settings-layout");
  layout.append(ledSettingsPanel());
  content.append(textElement("p", `${pool.model} · device settings`, "device-settings-model"), layout);
  return content;
}

function ledChoice(unitId: string): { desired: number | null; lastRaw: number | null; readAtMs: number | null } {
  let choice = ledChoices.get(unitId);
  if (!choice) {
    if (ledChoices.size >= 32) ledChoices.delete(ledChoices.keys().next().value as string);
    choice = { desired: null, lastRaw: null, readAtMs: null };
    ledChoices.set(unitId, choice);
  }
  return choice;
}

function ledSettingsPanel(): HTMLElement {
  const panel = element("section", "led-settings settings-card glass");
  panel.append(textElement("h3", "Dim Lights", "card-heading"));
  if (!controlStatus && !controlStatusLoading && !controlStatusError) window.queueMicrotask(() => void loadControlStatus());
  if (controlStatusError) panel.append(textElement("p", controlStatusError, "status-copy"));
  const target = selectedControlTarget();
  const setter = target?.controls.find((control) => control.action === "brightness");
  const reader = target?.controls.find((control) => control.action === "get-brightness");
  if (!target || !setter || !reader) {
    const selectableTarget = controlStatus?.targets.some((candidate) => candidate.id === selectedPoolId);
    panel.append(textElement("p", controlStatusLoading ? "Checking light-control availability…" : selectableTarget ? "Choose this device as the control target in Tools to use its light controls." : "Light controls are unavailable for the selected device.", "status-copy"));
    if (selectableTarget) panel.append(actionButton("Choose control target in Tools", "secondary", () => selectSection("tools")));
    return panel;
  }
  const choice = ledChoice(target.id);
  const rawLevels = [1, 2, 5, 7, 9, 11, 15, 25, 40, 59];
  const index = choice.lastRaw === null ? -1 : rawLevels.indexOf(choice.lastRaw);
  const current = choice.lastRaw === null ? "Current level: not read" : `Last read: ${index < 0 ? `raw level ${choice.lastRaw}` : `preset ${index} of 9`} · ${new Date(choice.readAtMs ?? 0).toLocaleTimeString()}`;
  panel.append(textElement("p", current, "status-copy"));
  const sliderLabel = textElement("label", "Desired light level", "field-label");
  const slider = document.createElement("input");
  slider.id = "led-preset-slider";
  slider.type = "range";
  slider.min = "0";
  slider.max = "9";
  slider.step = "1";
  slider.value = String(choice.desired ?? 5);
  slider.disabled = controlStatusError !== null || !setter.enabled || controlBusyAction !== null;
  slider.setAttribute("aria-valuetext", choice.desired === null ? "No desired preset selected" : `Desired preset ${choice.desired} of 9`);
  sliderLabel.htmlFor = slider.id;
  const selected = textElement("output", choice.desired === null ? "Choose a level" : `Desired preset ${choice.desired} of 9`, "led-desired");
  selected.setAttribute("for", slider.id);
  const set = actionButton(controlBusyAction === "brightness" ? "Applying…" : "Apply light level", "secondary", () => {
    if (choice.desired !== null) void executeControl(target.id, setter, choice.desired);
  });
  set.id = "led-apply";
  const updateChoice = () => {
    const value = Number(slider.value);
    if (!Number.isInteger(value) || value < 0 || value > 9) return;
    choice.desired = value;
    selected.value = `Desired preset ${value} of 9`;
    slider.setAttribute("aria-valuetext", selected.value);
    set.disabled = slider.disabled;
  };
  slider.addEventListener("input", updateChoice);
  slider.addEventListener("pointerup", updateChoice);
  slider.addEventListener("keyup", (event) => {
    if (["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End"].includes(event.key)) updateChoice();
  });
  set.disabled = slider.disabled || choice.desired === null;
  const scale = element("div", "led-scale");
  scale.append(textElement("span", "Dim · 0"), textElement("span", "Bright · 9"));
  sliderLabel.append(slider, scale, selected);
  const read = actionButton(controlBusyAction === "get-brightness" ? "Reading…" : "Read current level", "secondary", () => void executeControl(target.id, reader));
  read.id = "led-read";
  read.disabled = controlStatusError !== null || !reader.enabled || controlBusyAction !== null;
  const actions = element("div", "panel-actions");
  actions.append(read, set);
  panel.append(sliderLabel, actions, textElement("p", "Moving the slider selects a preset only. Apply sends the change. Presets are not percentages.", "status-copy"));
  if (!setter.enabled) panel.append(textElement("p", setter.reason, "status-copy"));
  if (controlResult && controlResultTargetId === target.id && ["brightness", "get-brightness"].includes(controlResult.action)) panel.append(controlResultCard(controlResult));
  return panel;
}

function disabledTextField(labelText: string, kind: "text" | "password" | "number" = "text"): HTMLLabelElement {
  const label = textElement("label", labelText, "field-label");
  const input = document.createElement("input");
  input.type = kind;
  input.disabled = true;
  input.placeholder = "Unavailable";
  input.autocomplete = "off";
  label.append(input);
  return label;
}

function disabledOptions(labelText: string, options: string[]): HTMLFieldSetElement {
  const field = element("fieldset", "settings-options");
  field.disabled = true;
  field.append(textElement("legend", labelText));
  for (const [index, option] of options.entries()) {
    const label = textElement("label", option, "radio-row");
    const input = document.createElement("input");
    input.type = "radio";
    input.name = `unavailable-${labelText}`;
    input.value = String(index);
    input.disabled = true;
    label.prepend(input);
    field.append(label);
  }
  return field;
}

function unavailableSettingButton(label: string): HTMLButtonElement {
  const button = actionButton(label, "secondary", () => undefined);
  button.disabled = true;
  return button;
}

function disabledCheckbox(labelText: string, state: string): HTMLLabelElement {
  const label = textElement("label", labelText, "toggle-row disabled-toggle"); const input = document.createElement("input"); input.type = "checkbox"; input.disabled = true; input.indeterminate = true; input.setAttribute("aria-checked", "mixed"); label.prepend(input); label.append(textElement("span", state, "status-copy")); return label;
}

function mcpPanel(content: HTMLElement): HTMLElement {
  content.append(panelIntro("MCP", "Let a trusted assistant or script inspect capacity, SSD life, events and sharing. OpenDrobo works without this connection."), mcpCard()); return content;
}

function helpPanel(content: HTMLElement): HTMLElement {
  content.append(panelIntro("Help", "Local read-only dashboard."), stateCard("Capability boundary", "Review Reference for supported and unavailable features. Controls are in Tools.", "quiet")); return content;
}

function panelIntro(title: string, subtitle: string): HTMLElement {
  const intro = element("div", "panel-intro");
  const heading = element("div", "panel-heading");
  heading.append(textElement("h2", title), textElement("p", subtitle));
  intro.append(heading);
  return intro;
}

function stateCard(title: string, message: string, tone: "quiet" | "error"): HTMLElement {
  const card = element("section", `state-card glass ${tone}`);
  card.append(textElement("h3", title), textElement("p", message));
  return card;
}

function liveStatus(title: string, message: string, tone: "quiet" | "error"): HTMLElement {
  const card = stateCard(title, message, tone);
  card.setAttribute("role", "status");
  card.setAttribute("aria-live", "polite");
  card.setAttribute("aria-atomic", "true");
  return card;
}

function statusStrip(): HTMLElement {
  const strip = element("footer", "status-strip");
  const platform = inventory?.platform ?? appInfo?.platform ?? "runtime unavailable";
  const message = poolStatus?.pools.length
    ? `${poolIsStale() ? "Last observation" : "Updated"} ${new Date(poolStatus.observedAtMs).toLocaleTimeString()}${poolIsStale() ? " · stale" : ""}`
    : inventory?.devices.length ? "Drobo discovered · device information pending" : "No Drobo connected";
  strip.append(textElement("span", platform), textElement("span", message));
  return strip;
}

function segmentedControl(label: string, values: Array<[string, boolean, () => void]>): HTMLElement {
  const group = element("div", "segmented");
  group.setAttribute("role", "group");
  group.setAttribute("aria-label", label);
  for (const [title, selected, action] of values) {
    const button = actionButton(title, selected ? "segment selected" : "segment", action);
    button.id = `segment-${label}-${title}`.toLowerCase().replace(/[^a-z0-9-]+/g, "-");
    button.setAttribute("aria-pressed", String(selected));
    group.append(button);
  }
  return group;
}

function actionButton(label: string, kind: "primary" | "secondary" | "segment" | "segment selected", onClick: () => void): HTMLButtonElement {
  const button = element("button", `action-button ${kind}`) as HTMLButtonElement;
  button.type = "button";
  button.textContent = label;
  button.addEventListener("click", onClick);
  return button;
}

function element<K extends keyof HTMLElementTagNameMap>(tag: K, className?: string): HTMLElementTagNameMap[K];
function element(tag: string, className?: string): HTMLElement {
  const node = document.createElement(tag);
  if (className) node.className = className;
  return node;
}

function textElement<K extends keyof HTMLElementTagNameMap>(tag: K, value: string, className?: string): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  if (className) node.className = className;
  node.textContent = value;
  return node;
}

function appendDefinition(list: HTMLElement, term: string, detail: string): void {
  list.append(textElement("dt", term), textElement("dd", detail));
}

function formatBytes(raw: string): string {
  try {
    const bytes = BigInt(raw);
    if (bytes < 0n) return "Unknown";
    const units = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];
    let value = bytes;
    let unit = 0;
    while (value >= 1024n && unit < units.length - 1) {
      value /= 1024n;
      unit += 1;
    }
    const divisor = 1024n ** BigInt(unit);
    const tenths = (bytes * 10n) / divisor;
    const whole = tenths / 10n;
    const decimal = tenths % 10n;
    return unit === 0 ? `${whole.toLocaleString()} ${units[unit]}` : `${whole.toLocaleString()}.${decimal} ${units[unit]}`;
  } catch {
    return "Unknown";
  }
}

function formatDecimalBytes(raw: string): string {
  try {
    const bytes = BigInt(raw);
    if (bytes < 0n) return "Unknown";
    const units = ["B", "KB", "MB", "GB", "TB", "PB", "EB"];
    let value = bytes;
    let unit = 0;
    while (value >= 1000n && unit < units.length - 1) {
      value /= 1000n;
      unit += 1;
    }
    const divisor = 1000n ** BigInt(unit);
    const hundredths = (bytes * 100n + divisor / 2n) / divisor;
    const whole = hundredths / 100n;
    const decimal = (hundredths % 100n).toString().padStart(2, "0");
    return unit === 0 ? `${whole.toLocaleString()} ${units[unit]}` : `${whole.toLocaleString()}.${decimal} ${units[unit]}`;
  } catch {
    return "Unknown";
  }
}

function safeBytes(raw: string): bigint {
  try { const value = BigInt(raw); return value < 0n ? 0n : value; } catch { return 0n; }
}

function capacityIsStale(): boolean {
  return capacityRequestedAtMs !== null && Date.now() - capacityRequestedAtMs >= capacityMaxAgeMs;
}

function clearCapacityExpiry(): void {
  if (capacityExpiryTimer !== null) window.clearTimeout(capacityExpiryTimer);
  capacityExpiryTimer = null;
}

function scheduleCapacityExpiry(requestToken: number): void {
  clearCapacityExpiry();
  capacityExpiryTimer = window.setTimeout(() => {
    capacityExpiryTimer = null;
    if (requestToken === capacityRequestSequence && selectedSection === "capacity") render();
  }, capacityMaxAgeMs);
}

function poolIsStale(): boolean {
  return poolStaleError !== null || (poolStatus !== null && Date.now() - poolStatus.observedAtMs >= capacityMaxAgeMs);
}

function clearPoolExpiry(): void {
  if (poolExpiryTimer !== null) window.clearTimeout(poolExpiryTimer);
  poolExpiryTimer = null;
}

function schedulePoolExpiry(requestToken: number): void {
  clearPoolExpiry();
  if (!poolStatus) return;
  const remaining = Math.max(0, capacityMaxAgeMs - (Date.now() - poolStatus.observedAtMs));
  poolExpiryTimer = window.setTimeout(() => {
    poolExpiryTimer = null;
    if (requestToken === poolRequestSequence) renderPoolConsumers();
  }, remaining);
}

function selectSection(section: Section): void {
  if (section !== selectedSection) clearPendingControl("Page changed; pending power confirmation was cancelled.");
  selectedSection = section;
  requestGeneration += 1;
  capacityRequestSequence += 1;
  updateRequestSequence += 1;
  capacityLoading = false;
  updateLoading = false;
  syncConsolePolling();
  syncEventsPolling();
  if (section === "tools" || section === "settings") void loadControlStatus();
  if (section === "statistics") { void loadCapacityHistory(); void loadDeviceEvents(true); }
  render();
}

async function loadCapacityHistory(): Promise<void> {
  if (capacityHistoryLoading) return;
  const requestToken = ++capacityHistoryRequestSequence;
  capacityHistoryLoading = true;
  capacityHistoryError = null;
  if (selectedSection === "statistics") render();
  try {
    const result = await call<CapacityHistoryResponse>("get_capacity_history", { limit: 2048 });
    if (requestToken !== capacityHistoryRequestSequence || selectedSection !== "statistics") return;
    capacityHistory = result;
  } catch (error) {
    if (requestToken !== capacityHistoryRequestSequence || selectedSection !== "statistics") return;
    capacityHistoryError = errorMessage(error);
  } finally {
    if (requestToken === capacityHistoryRequestSequence) capacityHistoryLoading = false;
  }
  if (requestToken === capacityHistoryRequestSequence && selectedSection === "statistics") render();
}

async function loadPoolStatus(showLoading: boolean): Promise<void> {
  if (poolLoading) return;
  const requestToken = ++poolRequestSequence;
  poolLoading = true;
  poolError = null;
  if (showLoading) renderPoolConsumers();
  try {
    const result = await call<PoolStatusResponse>("get_pool_status");
    if (requestToken !== poolRequestSequence) return;
    applyPoolSnapshot(result, requestToken);
  } catch (error) {
    if (requestToken !== poolRequestSequence) return;
    if (poolStatus) poolStaleError = errorMessage(error);
    else poolError = errorMessage(error);
    if (!poolStatus) clearPoolExpiry();
  } finally {
    if (requestToken === poolRequestSequence) poolLoading = false;
  }
  if (requestToken === poolRequestSequence) renderPoolConsumers();
}

function applyPoolSnapshot(snapshot: PoolStatusResponse, requestToken?: number): void {
  if (poolStatus && snapshot.observedAtMs < poolStatus.observedAtMs) return;
  if (requestToken === undefined) {
    poolRequestSequence += 1;
    poolLoading = false;
  }
  poolStatus = snapshot;
  let selectionLost = false;
  if (selectedPoolId && !snapshot.pools.some((pool) => pool.id === selectedPoolId)) {
    selectedPoolId = null;
    selectedControlTargetId = null;
    selectionLost = true;
    clearPendingControl("Selected pool disappeared; pending control confirmation was cancelled.");
  }
  poolError = null;
  poolStaleError = null;
  schedulePoolExpiry(requestToken ?? poolRequestSequence);
  if (selectionLost && (selectedSection === "tools" || selectedSection === "settings")) render();
}

function renderPoolConsumers(): void {
  if (selectedSection === "all" || selectedSection === "capacity" || selectedSection === "status") render();
}

async function subscribePoolEvents(): Promise<void> {
  if (poolEventSubscribed || !tauriAvailable()) return;
  poolEventSubscribed = true;
  await listen<PoolStatusResponse>("opendrobo-pool-status", (event) => {
    applyPoolSnapshot(event.payload);
    renderPoolConsumers();
  });
  await listen<string>("opendrobo-pool-error", (event) => {
    if (poolStatus) poolStaleError = event.payload;
    else poolError = event.payload;
    renderPoolConsumers();
  });
}

function captureConsoleDomState(): { focusId: string | null; selectionStart: number | null; selectionEnd: number | null; scrollTop: number | null } | null {
  if (selectedSection !== "console") return null;
  const active = document.activeElement;
  const focusableIds = new Set(["console-search", "console-level", "console-refresh", "console-pause", "console-copy", "console-events"]);
  const focusId = active instanceof HTMLElement && focusableIds.has(active.id) ? active.id : null;
  const input = active instanceof HTMLInputElement ? active : null;
  const events = document.getElementById("console-events");
  return {
    focusId,
    selectionStart: input?.selectionStart ?? null,
    selectionEnd: input?.selectionEnd ?? null,
    scrollTop: events?.scrollTop ?? null
  };
}

function restoreConsoleDomState(state: ReturnType<typeof captureConsoleDomState>): void {
  if (!state || selectedSection !== "console") return;
  window.requestAnimationFrame(() => {
    if (selectedSection !== "console") return;
    const events = document.getElementById("console-events");
    if (events && state.scrollTop !== null) events.scrollTop = state.scrollTop;
    if (!state.focusId) return;
    const target = document.getElementById(state.focusId);
    if (!(target instanceof HTMLElement)) return;
    target.focus({ preventScroll: true });
    if (target instanceof HTMLInputElement && state.selectionStart !== null && state.selectionEnd !== null) {
      target.setSelectionRange(state.selectionStart, state.selectionEnd);
    }
  });
}

function captureEventsDomState(): { focusId: string | null; selectionStart: number | null; selectionEnd: number | null; scrollTop: number | null } | null {
  if (selectedSection !== "events") return null;
  const active = document.activeElement;
  const focusableIds = new Set(["events-search", "events-severity", "events-refresh", "events-pause", "device-events"]);
  const focusId = active instanceof HTMLElement && focusableIds.has(active.id) ? active.id : null;
  const input = active instanceof HTMLInputElement ? active : null;
  const events = document.getElementById("device-events");
  return {
    focusId,
    selectionStart: input?.selectionStart ?? null,
    selectionEnd: input?.selectionEnd ?? null,
    scrollTop: events?.scrollTop ?? null
  };
}

function restoreEventsDomState(state: ReturnType<typeof captureEventsDomState>): void {
  if (!state || selectedSection !== "events") return;
  window.requestAnimationFrame(() => {
    if (selectedSection !== "events") return;
    const events = document.getElementById("device-events");
    if (events && state.scrollTop !== null) events.scrollTop = state.scrollTop;
    if (!state.focusId) return;
    const target = document.getElementById(state.focusId);
    if (!(target instanceof HTMLElement)) return;
    target.focus({ preventScroll: true });
    if (target instanceof HTMLInputElement && state.selectionStart !== null && state.selectionEnd !== null) {
      target.setSelectionRange(state.selectionStart, state.selectionEnd);
    }
  });
}

function captureManagementDomState(): { focusId: string | null; selectionStart: number | null; selectionEnd: number | null; scrollY: number } | null {
  if (selectedSection !== "tools") return null;
  const active = document.activeElement;
  const focusId = active instanceof HTMLElement && active.id ? active.id : null;
  const input = active instanceof HTMLInputElement ? active : null;
  return {
    focusId,
    selectionStart: input?.selectionStart ?? null,
    selectionEnd: input?.selectionEnd ?? null,
    scrollY: window.scrollY
  };
}

function restoreManagementDomState(state: ReturnType<typeof captureManagementDomState>): void {
  if (!state || selectedSection !== "tools") return;
  window.requestAnimationFrame(() => {
    if (selectedSection !== "tools") return;
    window.scrollTo({ top: state.scrollY });
    const focusId = managementFocusTargetId ?? state.focusId;
    if (!focusId) return;
    const target = document.getElementById(focusId);
    if (!(target instanceof HTMLElement)) return;
    target.focus({ preventScroll: true });
    if (target instanceof HTMLInputElement && state.selectionStart !== null && state.selectionEnd !== null) {
      target.setSelectionRange(state.selectionStart, state.selectionEnd);
    }
    managementFocusTargetId = null;
  });
}

function syncConsolePolling(): void {
  const shouldPoll = selectedSection === "console" && !consolePaused && document.visibilityState === "visible";
  if (!shouldPoll) {
    if (consolePollTimer !== null) window.clearInterval(consolePollTimer);
    consolePollTimer = null;
    return;
  }
  if (consolePollTimer !== null) return;
  consolePollTimer = window.setInterval(() => void loadConsoleRecords(false), 2_000);
}

function syncEventsPolling(): void {
  const shouldPoll = selectedSection === "events" && !eventsPaused && document.visibilityState === "visible";
  if (!shouldPoll) {
    if (eventsPollTimer !== null) window.clearInterval(eventsPollTimer);
    eventsPollTimer = null;
    return;
  }
  if (eventsPollTimer !== null) return;
  eventsPollTimer = window.setInterval(() => void loadDeviceEvents(false), 2_000);
}

async function loadStorageStatus(): Promise<void> {
  if (storageLoading) return;
  const sectionGeneration = requestGeneration;
  const requestToken = ++storageRequestSequence;
  storageLoading = true;
  storageError = null;
  render();
  try {
    const result = await call<StorageStatusResponse>("get_storage_status");
    if (sectionGeneration !== requestGeneration || requestToken !== storageRequestSequence || selectedSection !== "volumes") return;
    storageStatus = result;
  } catch (error) {
    if (sectionGeneration !== requestGeneration || requestToken !== storageRequestSequence || selectedSection !== "volumes") return;
    storageError = errorMessage(error);
  } finally {
    if (requestToken === storageRequestSequence) storageLoading = false;
  }
  if (requestToken === storageRequestSequence && selectedSection === "volumes") render();
}

async function loadSharingStatus(): Promise<void> {
  if (sharingLoading) return;
  const sectionGeneration = requestGeneration;
  const requestToken = ++sharingRequestSequence;
  sharingLoading = true;
  sharingError = null;
  render();
  try {
    const result = await call<SharingStatusResponse>("get_sharing_status");
    if (sectionGeneration !== requestGeneration || requestToken !== sharingRequestSequence || selectedSection !== "sharing") return;
    sharingStatus = result;
  } catch (error) {
    if (sectionGeneration !== requestGeneration || requestToken !== sharingRequestSequence || selectedSection !== "sharing") return;
    sharingError = errorMessage(error);
  } finally {
    if (requestToken === sharingRequestSequence) sharingLoading = false;
  }
  if (requestToken === sharingRequestSequence && selectedSection === "sharing") render();
}

async function openSharingSettings(): Promise<void> {
  if (selectedSection !== "sharing") return;
  const sectionGeneration = requestGeneration;
  try {
    await call<void>("open_sharing_settings");
    if (sectionGeneration !== requestGeneration || selectedSection !== "sharing") return;
    sharingSettingsStatus = "macOS File Sharing settings requested. Refresh after making native changes.";
  } catch (error) {
    if (sectionGeneration !== requestGeneration || selectedSection !== "sharing") return;
    sharingSettingsStatus = `Could not open macOS File Sharing settings: ${errorMessage(error)}`;
  }
  render();
}

async function loadConsoleRecords(showLoading: boolean): Promise<void> {
  if (consoleLoading || selectedSection !== "console") return;
  const sectionGeneration = requestGeneration;
  const requestToken = ++consoleRequestSequence;
  consoleLoading = true;
  consoleError = null;
  if (showLoading) render();
  try {
    const result = await call<ConsoleResponse>("get_console_records", { limit: 200 });
    if (sectionGeneration !== requestGeneration || requestToken !== consoleRequestSequence || selectedSection !== "console") return;
    consoleResponse = result;
  } catch (error) {
    if (sectionGeneration !== requestGeneration || requestToken !== consoleRequestSequence || selectedSection !== "console") return;
    consoleError = errorMessage(error);
  } finally {
    if (requestToken === consoleRequestSequence) consoleLoading = false;
  }
  if (requestToken === consoleRequestSequence && selectedSection === "console") render();
}

async function loadDeviceEvents(showLoading: boolean): Promise<void> {
  if (eventsLoading || (selectedSection !== "events" && selectedSection !== "statistics")) return;
  const sectionGeneration = requestGeneration;
  const requestToken = ++eventsRequestSequence;
  eventsLoading = true;
  eventsError = null;
  if (showLoading) render();
  try {
    const result = await call<DeviceEventsResponse>("get_device_events", { limit: selectedSection === "statistics" ? 500 : 200 });
    if (sectionGeneration !== requestGeneration || requestToken !== eventsRequestSequence || (selectedSection !== "events" && selectedSection !== "statistics")) return;
    eventsResponse = result;
  } catch (error) {
    if (sectionGeneration !== requestGeneration || requestToken !== eventsRequestSequence || (selectedSection !== "events" && selectedSection !== "statistics")) return;
    eventsError = errorMessage(error);
  } finally {
    if (requestToken === eventsRequestSequence) eventsLoading = false;
  }
  if (requestToken === eventsRequestSequence && (selectedSection === "events" || selectedSection === "statistics")) render();
}

async function loadControlStatus(): Promise<void> {
  if (controlStatusLoading) return;
  const sectionGeneration = requestGeneration;
  const requestToken = ++controlStatusRequestSequence;
  controlStatusLoading = true;
  controlStatusError = null;
  if (selectedSection === "tools" || selectedSection === "settings") render();
  try {
    const result = await call<ControlStatusResponse>("get_control_status");
    if (sectionGeneration !== requestGeneration || requestToken !== controlStatusRequestSequence || (selectedSection !== "tools" && selectedSection !== "settings")) return;
    controlStatus = result;
    if (selectedControlTargetId !== null && !result.targets.some((target) => target.id === selectedControlTargetId)) {
      clearPendingControl("Control target disappeared; pending confirmation was cancelled.");
      selectedControlTargetId = null;
      selectedPoolId = null;
    }
  } catch (error) {
    if (sectionGeneration !== requestGeneration || requestToken !== controlStatusRequestSequence || (selectedSection !== "tools" && selectedSection !== "settings")) return;
    controlStatusError = errorMessage(error);
    clearPendingControl("Control status is unavailable; pending confirmation was cancelled.");
    selectedControlTargetId = null;
  } finally {
    if (requestToken === controlStatusRequestSequence) controlStatusLoading = false;
  }
  if (requestToken === controlStatusRequestSequence && (selectedSection === "tools" || selectedSection === "settings")) render();
}

async function executeControl(unitId: string, control: DeviceControl, value?: number, confirmation?: string): Promise<void> {
  if (!control.enabled || controlBusyAction !== null) return;
  if (control.action === "brightness" && (control.minimum === null || control.maximum === null || value === undefined || !Number.isInteger(value) || value < control.minimum || value > control.maximum)) return;
  if (isPowerAction(control.action) && confirmation !== control.action.toUpperCase()) return;
  controlBusyAction = control.action;
  controlResult = null;
  controlResultTargetId = unitId;
  if (pendingControl?.action === control.action && pendingControlTargetId === unitId) managementFocusTargetId = "control-confirmation-status";
  render();
  try {
    const request: { unitId: string; action: ControlAction; value?: number; confirmation?: string } = { unitId, action: control.action };
    if (value !== undefined) request.value = value;
    if (confirmation !== undefined) request.confirmation = confirmation;
    const result = await call<ControlResult>("execute_device_control", { request });
    controlResult = result;
    if (result.state === "accepted" && control.action === "get-brightness" && typeof result.value === "number") {
      const choice = ledChoice(unitId);
      choice.lastRaw = result.value;
      choice.readAtMs = Date.now();
    } else if (result.state === "accepted" && control.action === "brightness") {
      const choice = ledChoice(unitId);
      choice.lastRaw = null;
      choice.readAtMs = null;
    }
  } catch (error) {
    controlResult = { state: "unknown", action: control.action, message: `Control request did not return a confirmed result: ${errorMessage(error)}` };
  } finally {
    controlBusyAction = null;
  }
  if (pendingControl?.action === control.action && pendingControlTargetId === unitId) {
    clearPendingControl();
    managementFocusTargetId = pendingControlLauncherId;
    pendingControlLauncherId = null;
  }
  if (selectedSection === "tools" || selectedSection === "settings") render();
}

async function copyConsoleRecords(records: ConsoleRecord[]): Promise<void> {
  const text = records.map((record) => `${formatConsoleTime(record.timestampMs)} [${record.level}] ${record.source}/${record.operation}: ${record.message}`).join("\n");
  await copyText(text, `${records.length} visible OpenDrobo event${records.length === 1 ? "" : "s"} copied.`, "console");
}

async function copyText(value: string, successMessage: string, scope: "console" | "mcp"): Promise<void> {
  if (!navigator.clipboard?.writeText) {
    setClipboardStatus(scope, "Clipboard access is unavailable in this desktop runtime.");
    render();
    return;
  }
  try {
    await navigator.clipboard.writeText(value);
    setClipboardStatus(scope, successMessage);
  } catch {
    setClipboardStatus(scope, "Clipboard access was not granted by this desktop runtime.");
  }
  render();
}

function setClipboardStatus(scope: "console" | "mcp", message: string): void {
  if (scope === "console") consoleClipboardStatus = message;
  else mcpClipboardStatus = message;
}

async function loadInventory(): Promise<void> {
  if (inventoryLoading) return;
  inventoryLoading = true;
  const generation = ++requestGeneration;
  inventoryError = null;
  render();
  try {
    const result = await call<InventoryResponse>("get_inventory");
    if (generation !== requestGeneration || selectedSection !== "all") return;
    inventory = result;
    if (!selectedDeviceId || !result.devices.some((device) => device.id === selectedDeviceId)) selectedDeviceId = result.devices[0]?.id ?? null;
  } catch (error) {
    if (generation !== requestGeneration || selectedSection !== "all") return;
    inventoryError = errorMessage(error);
  } finally {
    inventoryLoading = false;
    if (selectedSection === "all") render();
  }
}

async function readCapacity(): Promise<void> {
  if (!selectedDeviceId || capacityLoading) return;
  const generation = ++requestGeneration;
  const requestToken = ++capacityRequestSequence;
  const deviceId = selectedDeviceId;
  capacityLoading = true;
  render();
  try {
    const result = await call<CapacityResponse>("read_capacity", { deviceId });
    if (generation !== requestGeneration || requestToken !== capacityRequestSequence || selectedSection !== "capacity" || selectedDeviceId !== deviceId) return;
    capacity = result;
    capacityRequestedAtMs = Date.now();
    scheduleCapacityExpiry(requestToken);
  } catch (error) {
    if (generation !== requestGeneration || requestToken !== capacityRequestSequence || selectedSection !== "capacity" || selectedDeviceId !== deviceId) return;
    capacity = { deviceId, state: "error", protectedTotalBytes: null, protectedUsedBytes: null, protectedFreeBytes: null, observedAtMs: null, message: errorMessage(error) };
    capacityRequestedAtMs = Date.now();
    scheduleCapacityExpiry(requestToken);
  } finally {
    if (requestToken === capacityRequestSequence) capacityLoading = false;
  }
  if (generation === requestGeneration && requestToken === capacityRequestSequence && selectedSection === "capacity" && selectedDeviceId === deviceId) render();
}

async function checkUpdates(background = false): Promise<void> {
  if (updateLoading) return;
  const generation = ++requestGeneration;
  const requestToken = ++updateRequestSequence;
  updateLoading = true;
  render();
  try {
    const result = await call<UpdateResponse>("check_updates");
    if (generation !== requestGeneration || requestToken !== updateRequestSequence || (!background && selectedSection !== "preferences" && selectedSection !== "tools")) return;
    updateResult = result;
  } catch (error) {
    if (generation !== requestGeneration || requestToken !== updateRequestSequence || (!background && selectedSection !== "preferences" && selectedSection !== "tools")) return;
    updateResult = { state: "error", message: errorMessage(error), releaseUrl: null };
  } finally {
    if (requestToken === updateRequestSequence) updateLoading = false;
  }
  if (generation === requestGeneration && requestToken === updateRequestSequence && (selectedSection === "preferences" || selectedSection === "tools")) render();
}

async function openRelease(url: string): Promise<void> {
  if (!url) return;
  const generation = ++requestGeneration;
  try {
    await call<void>("open_release", { url });
  } catch (error) {
    if (generation !== requestGeneration || (selectedSection !== "preferences" && selectedSection !== "tools")) return;
    updateResult = { state: "error", message: errorMessage(error), releaseUrl: null };
    render();
  }
}

function updateTitle(state: UpdateState): string {
  if (state === "available") return "Update available";
  if (state === "current") return "Current version";
  if (state === "none") return "No releases published";
  return "Update check failed";
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function initialize(): Promise<void> {
  inventoryLoading = true;
  render();
  if (!tauriAvailable()) {
    inventoryLoading = false;
    inventoryError = "Desktop runtime unavailable. Browser preview has no device fixture, registry, or hardware access.";
    render();
    return;
  }
  void subscribePoolEvents();
  void loadPoolStatus(false);
  const generation = requestGeneration;
  try {
    try {
      const result = await call<AppInfoResponse>("get_app_info");
      if (generation !== requestGeneration) return;
      appInfo = result;
    } catch {
      // App information is supplementary; registry error remains surfaced below.
    }
    try {
      const result = await call<InventoryResponse>("get_inventory");
      if (generation !== requestGeneration) return;
      inventory = result;
      selectedDeviceId = inventory.devices[0]?.id ?? null;
    } catch (error) {
      if (generation !== requestGeneration) return;
      inventoryError = errorMessage(error);
    }
    if (autoUpdateChecks) void checkUpdates(true);
  } finally {
    inventoryLoading = false;
    if (selectedSection === "all") render();
  }
}

document.addEventListener("visibilitychange", () => {
  syncConsolePolling();
  syncEventsPolling();
});
void initialize();
