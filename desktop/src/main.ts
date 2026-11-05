import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

type Section = "all" | "status" | "capacity" | "volumes" | "tools" | "settings" | "preferences" | "help";
type CapacityState = "available" | "unavailable" | "error";
type UpdateState = "none" | "available" | "current" | "error";

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
  message: string;
}

const sections: Array<{ id: Section; label: string; icon: string; artwork: string; activeArtwork: string }> = [
  { id: "all", label: "All Drobos", icon: "▦", artwork: "DroboNav-all_devices.png", activeArtwork: "DroboNav-all_devicesA.png" },
  { id: "status", label: "Status", icon: "♥", artwork: "DroboNav-status.png", activeArtwork: "DroboNav-statusA.png" },
  { id: "capacity", label: "Capacity", icon: "▣", artwork: "DroboNav-capacity.png", activeArtwork: "DroboNav-capacityA.png" },
  { id: "volumes", label: "Volumes", icon: "▤", artwork: "DroboNav-shares_volumes.png", activeArtwork: "DroboNav-shares_volumes_active.png" },
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
let inventory: InventoryResponse | null = null;
let inventoryError: string | null = null;
let inventoryLoading = false;
let capacity: CapacityResponse | null = null;
let capacityRequestedAtMs: number | null = null;
let capacityLoading = false;
let updateResult: UpdateResponse | null = null;
let updateLoading = false;
let appInfo: AppInfoResponse | null = null;
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
  app.replaceChildren();
  const shell = element("main", "app-shell");
  shell.append(header(), element("div", "rainbow-rule"));
  shell.append(beautyLineup());
  const body = element("div", "shell-body");
  body.append(sidebar(), panel());
  shell.append(body);
  app.append(shell);
}

function header(): HTMLElement {
  const headerElement = element("header", "topbar");
  const brand = element("div", "brand");
  brand.append(textElement("strong", "OpenDrobo"), textElement("span", "READ-ONLY PREVIEW"));
  const title = textElement("h1", "Drobo Dashboard");
  const metadata = textElement("div", appInfo ? `v${appInfo.version} · ${appInfo.platform}` : "read-only local dashboard", "top-meta");
  headerElement.append(brand, title, metadata);
  return headerElement;
}

function sidebar(): HTMLElement {
  const aside = element("aside", "sidebar glass");
  const nav = element("nav", "nav-list");
  nav.setAttribute("aria-label", "Dashboard sections");
  for (const section of sections) {
    const button = element("button", "nav-item");
    button.type = "button";
    button.classList.toggle("selected", selectedSection === section.id);
    button.setAttribute("aria-current", selectedSection === section.id ? "page" : "false");
    const icon = textElement("span", section.icon, "nav-icon");
    const art = document.createElement("img");
    art.className = "nav-art";
    art.src = `/local-artwork/${selectedSection === section.id ? section.activeArtwork : section.artwork}`;
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
    button.append(icon, textElement("span", section.label));
    button.addEventListener("click", () => selectSection(section.id));
    nav.append(button);
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
  footer.append(textElement("strong", "OpenDrobo"), textElement("span", "Discovery registry only"));
  return footer;
}

function panel(): HTMLElement {
  const content = element("section", "content");
  content.setAttribute("aria-live", "polite");
  switch (selectedSection) {
    case "all": return allDevicesPanel(content);
    case "capacity": return capacityPanel(content);
    case "preferences": return preferencesPanel(content);
    case "status":
    case "volumes":
    case "tools":
    case "settings":
    case "help": return unavailablePanel(content, selectedSection);
  }
}

function allDevicesPanel(content: HTMLElement): HTMLElement {
  const intro = panelIntro("All Drobos", "Registry endpoints only. Discovery does not establish health, capacity, storage access, or control.");
  const actions = element("div", "panel-actions");
  const refresh = actionButton(inventoryLoading ? "Refreshing…" : "Refresh registry", "primary", () => void loadInventory());
  refresh.disabled = inventoryLoading;
  actions.append(refresh, segmentedControl("Device view", [
    ["Icons", viewMode === "icons", () => { viewMode = "icons"; render(); }],
    ["List", viewMode === "list", () => { viewMode = "list"; render(); }]
  ]));
  intro.append(actions);
  content.append(intro);

  if (inventoryError) {
    content.append(stateCard("Registry unavailable", inventoryError, "error"));
  } else if (!inventory) {
    content.append(stateCard("Checking local registry", "No device data shown until local registry responds.", "quiet"));
  } else if (inventory.devices.length === 0) {
    content.append(stateCard("No registered Drobos", inventory.message || "Local registry returned no endpoints.", "quiet"));
  } else {
    const devices = [...inventory.devices].sort((a, b) => a.name.localeCompare(b.name));
    const list = element("div", viewMode === "icons" ? "device-grid" : "device-list");
    for (const device of devices) list.append(deviceCard(device));
    content.append(list, textElement("p", inventory.message, "status-copy"));
  }
  content.append(statusStrip());
  return content;
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
  const intro = panelIntro("Capacity", "Read-only protected-capacity data. Nothing is mounted, activated, written, or inferred.");
  content.append(intro);
  const devices = inventory?.devices ?? [];
  if (devices.length === 0) {
    content.append(stateCard("No selected device", "Return to All Drobos and refresh local registry first.", "quiet"));
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
  content.append(selector);
  content.append(capacityCard());
  return content;
}

function capacityCard(): HTMLElement {
  const card = element("section", "capacity-card glass");
  const stale = capacityIsStale();
  if (!capacity) return stateCard("Capacity unknown", "Select device, then use Read capacity. No hardware read runs on launch.", "quiet");
  if (stale) return stateCard("Capacity expired", "Last read is older than 30 seconds. Read again for current protected-capacity data.", "quiet");
  if (capacity.state !== "available") return stateCard(capacity.state === "error" ? "Capacity read failed" : "Capacity unavailable", capacity.message, capacity.state === "error" ? "error" : "quiet");
  const values: Array<[string, string | null]> = [
    ["Protected total", capacity.protectedTotalBytes],
    ["Protected used", capacity.protectedUsedBytes],
    ["Protected free", capacity.protectedFreeBytes]
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
  settings.append(label, textElement("p", "Uses local preference only. Checks OpenDrobo GitHub releases when enabled or manually requested.", "status-copy"));
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
  return content;
}

function unavailablePanel(content: HTMLElement, section: Exclude<Section, "all" | "capacity" | "preferences">): HTMLElement {
  const name = sections.find((entry) => entry.id === section)?.label ?? "Dashboard";
  content.append(panelIntro(name, "Original dashboard layout retained. This read-only preview does not claim controls not implemented by local core."));
  const messages: Record<Exclude<Section, "all" | "capacity" | "preferences">, string> = {
    status: "Health data is unavailable. Registry discovery alone cannot establish device health.",
    volumes: "Volume information is unavailable. This preview does not mount, browse, or change volumes.",
    tools: "No maintenance tools are available. Driver activation, firmware actions, and writes are intentionally absent.",
    settings: "Drobo settings are unavailable. This preview has no device configuration controls.",
    help: "OpenDrobo is a read-only local preview. Check project documentation for current capability boundaries."
  };
  content.append(stateCard(`${name} unavailable`, messages[section], "quiet"));
  return content;
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

function statusStrip(): HTMLElement {
  const strip = element("footer", "status-strip");
  const platform = inventory?.platform ?? appInfo?.platform ?? "runtime unavailable";
  const message = inventory?.message ?? inventoryError ?? "Browser preview: no device fixture or hardware access.";
  strip.append(textElement("span", platform), textElement("span", message));
  return strip;
}

function segmentedControl(label: string, values: Array<[string, boolean, () => void]>): HTMLElement {
  const group = element("div", "segmented");
  group.setAttribute("role", "group");
  group.setAttribute("aria-label", label);
  for (const [title, selected, action] of values) {
    const button = actionButton(title, selected ? "segment selected" : "segment", action);
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

function textElement(tag: string, value: string, className?: string): HTMLElement {
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

function selectSection(section: Section): void {
  selectedSection = section;
  requestGeneration += 1;
  capacityRequestSequence += 1;
  updateRequestSequence += 1;
  capacityLoading = false;
  updateLoading = false;
  render();
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
    if (generation !== requestGeneration || requestToken !== updateRequestSequence || (!background && selectedSection !== "preferences")) return;
    updateResult = result;
  } catch (error) {
    if (generation !== requestGeneration || requestToken !== updateRequestSequence || (!background && selectedSection !== "preferences")) return;
    updateResult = { state: "error", message: errorMessage(error), releaseUrl: null };
  } finally {
    if (requestToken === updateRequestSequence) updateLoading = false;
  }
  if (generation === requestGeneration && requestToken === updateRequestSequence && selectedSection === "preferences") render();
}

async function openRelease(url: string): Promise<void> {
  if (!url) return;
  const generation = ++requestGeneration;
  try {
    await call<void>("open_release", { url });
  } catch (error) {
    if (generation !== requestGeneration || selectedSection !== "preferences") return;
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

void initialize();
