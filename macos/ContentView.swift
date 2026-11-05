import AppKit
import SwiftUI

/// Native dashboard shell. Registry discovery proves only that macOS reported
/// an endpoint; it never establishes health, capacity, or control access.
struct DashboardRootView: View {
    fileprivate enum Section: String, CaseIterable, Identifiable {
        case allDrobos = "All Drobos"
        case status = "Status"
        case capacity = "Capacity"
        case volumes = "Volumes"
        case tools = "Tools"
        case settings = "Drobo Settings"
        case preferences = "Dashboard Preferences"
        case help = "Help and Support"

        var id: String { rawValue }

        var symbol: String {
            switch self {
            case .allDrobos: "square.grid.2x2"
            case .status: "heart.text.square"
            case .capacity: "internaldrive"
            case .volumes: "externaldrive"
            case .tools: "wrench.and.screwdriver"
            case .settings: "gearshape"
            case .preferences: "slider.horizontal.3"
            case .help: "questionmark.circle"
            }
        }

        var artworkStem: String {
            switch self {
            case .allDrobos: "DroboNav-all_devices"
            case .status: "DroboNav-status"
            case .capacity: "DroboNav-capacity"
            case .volumes: "DroboNav-shares_volumes"
            case .tools: "DroboNav-tools"
            case .settings: "DroboNav-settings"
            case .preferences: "DroboNav-prefs"
            case .help: "DroboNav-help"
            }
        }

        var activeArtworkStem: String {
            switch self {
            case .volumes: "DroboNav-shares_volumes_active"
            default: "\(artworkStem)A"
            }
        }
    }

    private enum ViewMode: String {
        case icons = "Icons"
        case list = "List"
    }

    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.scenePhase) private var scenePhase
    @State private var selection: Section = .allDrobos
    @State private var viewMode: ViewMode = .icons
    @State private var devices: [LocalDrobo] = []
    @State private var discoveryState: DiscoveryState = .loading
    @StateObject private var updater = ReleaseUpdater()

    var body: some View {
        ZStack {
            DashboardBackdrop(reduceTransparency: reduceTransparency)

            VStack(spacing: 0) {
                header
                HStack(spacing: 0) {
                    sidebar
                    Divider().overlay(Color.white.opacity(0.14))
                    mainPanel
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .frame(minWidth: 900, minHeight: 650)
        .preferredColorScheme(.dark)
        .transaction { transaction in
            if reduceMotion || scenePhase != .active {
                transaction.animation = nil
            }
        }
        .task {
            await refreshDiscovery()
            if updater.automaticChecks {
                await updater.check()
            }
        }
        .onChange(of: updater.automaticChecks) { _, enabled in
            if enabled {
                Task { await updater.check() }
            }
        }
    }

    private var header: some View {
        HStack {
            VStack(alignment: .leading, spacing: -2) {
                Text("OpenDrobo")
                    .font(.system(size: 26, weight: .black, design: .rounded))
                    .tracking(-1.2)
                Text("NATIVE PREVIEW")
                    .font(.system(size: 8, weight: .bold, design: .monospaced))
                    .tracking(1.5)
                    .foregroundStyle(Color.white.opacity(0.60))
            }
            .frame(width: 250, alignment: .leading)

            Spacer()

            Text("Drobo Dashboard")
                .font(.system(size: 18, weight: .semibold))
                .foregroundStyle(Color.white.opacity(0.96))

            Spacer()
            Color.clear.frame(width: 250, height: 1)
        }
        .padding(.horizontal, 22)
        .frame(height: 60)
        .dashboardGlass(cornerRadius: 0, tint: DashboardPalette.headerTint, reduceTransparency: reduceTransparency)
        .overlay(alignment: .bottom) {
            LinearGradient(
                colors: [DashboardPalette.cyan, DashboardPalette.violet, DashboardPalette.magenta],
                startPoint: .leading,
                endPoint: .trailing
            )
            .opacity(reduceTransparency ? 0.34 : 0.58)
            .frame(height: 1)
        }
    }

    private var sidebar: some View {
        VStack(spacing: 0) {
            VStack(spacing: 3) {
                ForEach(Section.allCases) { section in
                    SidebarRow(
                        section: section,
                        isSelected: selection == section,
                        reduceTransparency: reduceTransparency
                    ) {
                        selection = section
                    }
                }
            }
            .padding(10)

            Spacer(minLength: 8)

            VStack(spacing: 10) {
                ProductArtwork(maxHeight: 130, label: "Reference Drobo product artwork")
                    .frame(maxWidth: .infinity)
                Text("Reference artwork")
                    .font(.caption2)
                    .foregroundStyle(Color.white.opacity(0.45))

                VStack(alignment: .leading, spacing: 4) {
                    Text("OpenDrobo")
                        .font(.caption.weight(.semibold))
                    Text("Native preview · discovery only")
                        .font(.caption2)
                        .foregroundStyle(Color.white.opacity(0.58))
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .padding(16)
            .dashboardGlass(cornerRadius: 12, tint: DashboardPalette.sidebarTint, reduceTransparency: reduceTransparency)
            .padding(10)
        }
        .frame(width: 260)
        .dashboardGlass(cornerRadius: 0, tint: DashboardPalette.sidebarTint, reduceTransparency: reduceTransparency)
    }

    @ViewBuilder
    private var mainPanel: some View {
        switch selection {
        case .allDrobos:
            allDrobosPanel
        case .preferences:
            preferencesPanel
        case .help:
            supportPanel
        case .status, .capacity, .volumes, .tools, .settings:
            unavailablePanel(for: selection)
        }
    }

    private var allDrobosPanel: some View {
        VStack(spacing: 0) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 12) {
                    Text("All Drobos")
                        .font(.system(size: 25, weight: .light))

                    VStack(alignment: .leading, spacing: 6) {
                        Text("Sort By")
                            .font(.caption.weight(.medium))
                            .foregroundStyle(Color.white.opacity(0.65))
                        HStack(spacing: 4) {
                            RGBSegmentButton(
                                title: "Health",
                                isSelected: false,
                                isEnabled: false,
                                accessibilityHint: "Unavailable because discovery does not provide health data.",
                                action: {}
                            )
                            RGBSegmentButton(
                                title: "Name",
                                isSelected: true,
                                isEnabled: true,
                                accessibilityHint: "Sorts discovered endpoints by name.",
                                action: {}
                            )
                            RGBSegmentButton(
                                title: "Product",
                                isSelected: false,
                                isEnabled: false,
                                accessibilityHint: "Unavailable because discovery does not provide verified product data.",
                                action: {}
                            )
                        }
                        .padding(3)
                        .dashboardGlass(cornerRadius: 8, tint: DashboardPalette.controlTint, reduceTransparency: reduceTransparency)

                        Text("Name sorting only. Health and product records are not available.")
                            .font(.caption2)
                            .foregroundStyle(Color.white.opacity(0.48))
                    }
                }

                Spacer()

                VStack(alignment: .trailing, spacing: 10) {
                    Button("Refresh") {
                        Task { await refreshDiscovery() }
                    }
                    .buttonStyle(DashboardButtonStyle(reduceTransparency: reduceTransparency))
                    .disabled(discoveryState == .loading)

                    HStack(spacing: 4) {
                        RGBSegmentButton(
                            title: "Icons",
                            isSelected: viewMode == .icons,
                            isEnabled: !devices.isEmpty,
                            accessibilityHint: devices.isEmpty ? "Unavailable until a local endpoint is discovered." : "Displays discovered endpoints as cards.",
                            action: { viewMode = .icons }
                        )
                        RGBSegmentButton(
                            title: "List",
                            isSelected: viewMode == .list,
                            isEnabled: !devices.isEmpty,
                            accessibilityHint: devices.isEmpty ? "Unavailable until a local endpoint is discovered." : "Displays discovered endpoints as a list.",
                            action: { viewMode = .list }
                        )
                    }
                    .padding(3)
                    .dashboardGlass(cornerRadius: 8, tint: DashboardPalette.controlTint, reduceTransparency: reduceTransparency)
                    .accessibilityElement(children: .contain)
                    .accessibilityLabel("Device display style")
                }
            }
            .padding(.horizontal, 44)
            .padding(.top, 42)

            Spacer(minLength: 28)
            discoveryContent
                .padding(.horizontal, 44)
            Spacer(minLength: 28)
            statusStrip
                .padding(.horizontal, 28)
                .padding(.bottom, 22)
        }
        .dashboardMainSurface(reduceTransparency: reduceTransparency)
    }

    @ViewBuilder
    private var discoveryContent: some View {
        switch discoveryState {
        case .loading:
            informationCard(
                title: "Looking for Drobos",
                detail: "Reading local macOS registry metadata. No device commands are sent."
            ) {
                ProgressView().controlSize(.small)
            }
        case .failed(let message):
            informationCard(title: "Discovery unavailable", detail: message) {
                Image(systemName: "exclamationmark.triangle.fill")
                    .foregroundStyle(.yellow)
            }
        case .empty:
            emptyDiscoveryCard
        case .found:
            deviceCollection
        }
    }

    private var emptyDiscoveryCard: some View {
        HStack(spacing: 26) {
            ProductArtwork(maxHeight: 145, label: "Reference Drobo product artwork")
                .frame(width: 180)

            VStack(alignment: .leading, spacing: 10) {
                Label("No Drobos Detected", systemImage: "info.circle.fill")
                    .font(.system(size: 18, weight: .medium))
                    .foregroundStyle(DashboardPalette.info)
                Text("No supported Drobo endpoint was reported by this Mac. Connect hardware, then refresh.")
                    .font(.system(size: 13))
                Text("Artwork is a product reference only. Detection does not provide management support.")
                    .font(.system(size: 12))
                    .foregroundStyle(Color.white.opacity(0.58))
            }
            Spacer(minLength: 0)
        }
        .padding(28)
        .frame(maxWidth: 650, alignment: .leading)
        .dashboardGlass(cornerRadius: 16, tint: DashboardPalette.cardTint, reduceTransparency: reduceTransparency)
    }

    private func informationCard<Accessory: View>(
        title: String,
        detail: String,
        @ViewBuilder accessory: () -> Accessory
    ) -> some View {
        HStack(alignment: .top, spacing: 20) {
            accessory()
                .frame(width: 34, alignment: .center)
            VStack(alignment: .leading, spacing: 8) {
                Text(title)
                    .font(.system(size: 17, weight: .medium))
                Text(detail)
                    .font(.system(size: 13))
                    .foregroundStyle(Color.white.opacity(0.67))
                    .fixedSize(horizontal: false, vertical: true)
            }
            Spacer(minLength: 0)
        }
        .padding(28)
        .frame(maxWidth: 600, alignment: .leading)
        .dashboardGlass(cornerRadius: 16, tint: DashboardPalette.cardTint, reduceTransparency: reduceTransparency)
    }

    @ViewBuilder
    private var deviceCollection: some View {
        if viewMode == .icons {
            LazyVGrid(
                columns: [GridItem(.adaptive(minimum: 240, maximum: 330), spacing: 14)],
                spacing: 14
            ) {
                ForEach(sortedDevices) { device in
                    DeviceCard(device: device, reduceTransparency: reduceTransparency)
                }
            }
            .frame(maxWidth: 720, alignment: .leading)
        } else {
            VStack(alignment: .leading, spacing: 0) {
                ForEach(Array(sortedDevices.enumerated()), id: \.element.id) { index, device in
                    DeviceRow(device: device)
                        .padding(18)
                    if index < sortedDevices.count - 1 {
                        Divider().overlay(Color.white.opacity(0.16))
                    }
                }
            }
            .frame(maxWidth: 720)
            .dashboardGlass(cornerRadius: 16, tint: DashboardPalette.cardTint, reduceTransparency: reduceTransparency)
        }
    }

    private var sortedDevices: [LocalDrobo] {
        devices.sorted {
            $0.name.localizedStandardCompare($1.name) == .orderedAscending
        }
    }

    private var statusStrip: some View {
        HStack(spacing: 9) {
            Image(systemName: "circle.fill")
                .font(.system(size: 8))
                .foregroundStyle(DashboardPalette.cyan)
                .shadow(color: DashboardPalette.cyan.opacity(reduceTransparency ? 0 : 0.9), radius: 6)
            Text("Native preview")
            Text("•")
            Text("Local registry discovery only")
                .foregroundStyle(Color.white.opacity(0.64))
            Spacer()
            Text("No management transport")
                .foregroundStyle(Color.white.opacity(0.52))
        }
        .font(.system(size: 12))
        .padding(.horizontal, 14)
        .frame(height: 32)
        .dashboardGlass(cornerRadius: 10, tint: DashboardPalette.controlTint, reduceTransparency: reduceTransparency)
    }

    private func unavailablePanel(for section: Section) -> some View {
        VStack(spacing: 0) {
            Text(section.rawValue)
                .font(.system(size: 25, weight: .light))
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 44)
                .padding(.top, 42)
            Spacer()
            informationCard(
                title: "\(section.rawValue) unavailable",
                detail: "This native preview can discover registry metadata only. \(section.rawValue) stays disabled until a verified management transport exists."
            ) {
                Image(systemName: "lock.fill")
                    .font(.system(size: 27))
                    .foregroundStyle(Color.white.opacity(0.48))
            }
            Spacer()
            statusStrip
                .padding(.horizontal, 28)
                .padding(.bottom, 22)
        }
        .dashboardMainSurface(reduceTransparency: reduceTransparency)
    }

    private var preferencesPanel: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                Text("Dashboard Preferences")
                    .font(.system(size: 25, weight: .light))
                    .padding(.bottom, 4)

                PreferenceGroup(title: "Drobo Discovery", reduceTransparency: reduceTransparency) {
                    Text("This preview reads local registry metadata when you open or refresh All Drobos.")
                    Text("Discovery can identify an endpoint. It cannot read capacity, health, firmware, or settings.")
                        .foregroundStyle(Color.white.opacity(0.62))
                }

                PreferenceGroup(title: "Update Settings", reduceTransparency: reduceTransparency) {
                    Toggle("Automatically check for OpenDrobo updates", isOn: $updater.automaticChecks)
                        .toggleStyle(.switch)
                    HStack {
                        Button("Check Now") {
                            Task { await updater.check() }
                        }
                        .buttonStyle(DashboardButtonStyle(reduceTransparency: reduceTransparency))
                        .disabled(updater.isChecking)
                        if updater.isChecking {
                            ProgressView().controlSize(.small)
                        }
                        Text(updater.message)
                            .foregroundStyle(Color.white.opacity(0.62))
                            .fixedSize(horizontal: false, vertical: true)
                        if let release = updater.availableRelease {
                            Button("View Release") {
                                NSWorkspace.shared.open(release.pageURL)
                            }
                            .buttonStyle(DashboardButtonStyle(reduceTransparency: reduceTransparency))
                        }
                    }
                }

                PreferenceGroup(title: "Onscreen Alerts", reduceTransparency: reduceTransparency) {
                    Text("No live alerts. Device management transport is not available in this preview.")
                        .foregroundStyle(Color.white.opacity(0.62))
                }

                PreferenceGroup(title: "Email Alerts", reduceTransparency: reduceTransparency) {
                    Text("Email alert setup is unavailable. No account or mail settings are stored by this preview.")
                        .foregroundStyle(Color.white.opacity(0.62))
                }

                PreferenceGroup(title: "Other Settings", reduceTransparency: reduceTransparency) {
                    Text("Hardware settings, firmware controls, and driver activation are intentionally unavailable.")
                        .foregroundStyle(Color.white.opacity(0.62))
                }
            }
            .padding(44)
            .frame(maxWidth: 760, alignment: .leading)
        }
        .dashboardMainSurface(reduceTransparency: reduceTransparency)
    }

    private var supportPanel: some View {
        VStack(spacing: 0) {
            Text("Help and Support")
                .font(.system(size: 25, weight: .light))
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 44)
                .padding(.top, 42)
            Spacer()
            informationCard(
                title: "OpenDrobo native preview",
                detail: "This app does not contact legacy Drobo services or install drivers. Use local documentation and release notes for project support."
            ) {
                Image(systemName: "questionmark.circle.fill")
                    .font(.system(size: 30))
                    .foregroundStyle(DashboardPalette.info)
            }
            Spacer()
            statusStrip
                .padding(.horizontal, 28)
                .padding(.bottom, 22)
        }
        .dashboardMainSurface(reduceTransparency: reduceTransparency)
    }

    @MainActor
    private func refreshDiscovery() async {
        discoveryState = .loading
        let result = await Task.detached(priority: .userInitiated) {
            Result { try LocalDiscovery.scan() }
        }.value

        switch result {
        case .success(let found):
            devices = found
            discoveryState = found.isEmpty ? .empty : .found
        case .failure(let error):
            devices = []
            discoveryState = .failed(error.localizedDescription)
        }
    }
}

private enum DiscoveryState: Equatable {
    case loading
    case empty
    case found
    case failed(String)
}

private struct SidebarRow: View {
    let section: DashboardRootView.Section
    let isSelected: Bool
    let reduceTransparency: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 11) {
                NavigationArtwork(section: section, isSelected: isSelected)
                    .frame(width: 18, height: 18)
                Text(section.rawValue)
                Spacer()
            }
            .font(.system(size: 13))
            .foregroundStyle(isSelected ? Color.white : Color.white.opacity(0.74))
            .padding(.horizontal, 16)
            .frame(height: 34)
            .dashboardGlass(
                cornerRadius: 8,
                tint: isSelected ? DashboardPalette.selectionTint : .clear,
                reduceTransparency: reduceTransparency
            )
            .overlay {
                if isSelected {
                    AnimatedRGBEdge(cornerRadius: 8, intensity: 0.90)
                }
            }
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(isSelected ? .isSelected : [])
    }
}

private struct RGBSegmentButton: View {
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
    let title: String
    let isSelected: Bool
    let isEnabled: Bool
    let accessibilityHint: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Text(title)
                .font(.system(size: 12, weight: isSelected ? .semibold : .regular))
                .foregroundStyle(isEnabled ? Color.white.opacity(isSelected ? 1 : 0.72) : Color.white.opacity(0.34))
                .frame(minWidth: 52)
                .padding(.horizontal, 8)
                .frame(height: 24)
                .background {
                    if isSelected {
                        RoundedRectangle(cornerRadius: 5)
                            .fill(DashboardPalette.selectionTint.opacity(0.72))
                    }
                }
                .overlay {
                    if isSelected {
                        AnimatedRGBEdge(cornerRadius: 5, intensity: 0.86)
                    }
                }
        }
        .buttonStyle(.plain)
        .disabled(!isEnabled)
        .accessibilityHint(accessibilityHint)
        .accessibilityAddTraits(isSelected ? .isSelected : [])
    }
}

private struct DeviceCard: View {
    let device: LocalDrobo
    let reduceTransparency: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Image(systemName: "externaldrive.connected.to.line.below")
                    .font(.title2)
                    .foregroundStyle(DashboardPalette.info)
                Spacer()
                Text("Management pending")
                    .font(.caption.weight(.medium))
                    .foregroundStyle(Color.white.opacity(0.52))
            }
            Text(device.name)
                .font(.system(size: 14, weight: .medium))
            Text(device.layer)
                .font(.caption)
                .foregroundStyle(Color.white.opacity(0.62))
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(18)
        .dashboardGlass(cornerRadius: 14, tint: DashboardPalette.cardTint, reduceTransparency: reduceTransparency)
    }
}

private struct DeviceRow: View {
    let device: LocalDrobo

    var body: some View {
        HStack(spacing: 14) {
            Image(systemName: "externaldrive.connected.to.line.below")
                .font(.title2)
                .foregroundStyle(DashboardPalette.info)
            VStack(alignment: .leading, spacing: 3) {
                Text(device.name)
                    .font(.system(size: 14, weight: .medium))
                Text(device.layer)
                    .font(.caption)
                    .foregroundStyle(Color.white.opacity(0.62))
            }
            Spacer()
            Text("Management pending")
                .font(.caption.weight(.medium))
                .foregroundStyle(Color.white.opacity(0.52))
        }
    }
}

private struct PreferenceGroup<Content: View>: View {
    let title: String
    let reduceTransparency: Bool
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 13) {
            Text(title)
                .font(.system(size: 14, weight: .semibold))
            content.font(.system(size: 13))
        }
        .padding(18)
        .frame(maxWidth: .infinity, alignment: .leading)
        .dashboardGlass(cornerRadius: 14, tint: DashboardPalette.cardTint, reduceTransparency: reduceTransparency)
    }
}

private struct DashboardButtonStyle: ButtonStyle {
    @Environment(\.isEnabled) private var isEnabled
    let reduceTransparency: Bool

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 12, weight: .medium))
            .padding(.horizontal, 12)
            .frame(height: 27)
            .foregroundStyle(Color.white.opacity(configuration.isPressed ? 0.68 : 0.94))
            .dashboardGlass(
                cornerRadius: 7,
                tint: DashboardPalette.controlTint.opacity(configuration.isPressed ? 0.55 : 1),
                reduceTransparency: reduceTransparency
            )
            .overlay {
                if isEnabled {
                    AnimatedRGBEdge(cornerRadius: 7, intensity: 0.34)
                }
            }
    }
}

private struct ProductArtwork: View {
    let maxHeight: CGFloat
    let label: String

    var body: some View {
        Group {
            if let image = ArtworkLoader.productImage {
                Image(nsImage: image)
                    .resizable()
                    .scaledToFit()
            } else {
                Image(systemName: "externaldrive.fill")
                    .resizable()
                    .scaledToFit()
                    .symbolRenderingMode(.hierarchical)
                    .foregroundStyle(Color.white.opacity(0.34))
                    .padding(20)
            }
        }
        .frame(maxHeight: maxHeight)
        .accessibilityLabel(label)
    }
}

private enum ArtworkLoader {
    static func image(named name: String, fileExtension: String) -> NSImage? {
        guard let url = Bundle.main.url(forResource: name, withExtension: fileExtension) else {
            return nil
        }
        return NSImage(contentsOf: url)
    }

    static let productImage: NSImage? = {
        let candidates = [
            ("Drobo5DArtwork", "png"),
            ("Drobo5DArtwork", "icns"),
            ("DroboDASArtwork", "png"),
            ("DroboNASArtwork", "png"),
            ("DroboOriginalApp", "icns")
        ]
        for (name, ext) in candidates {
            if let image = image(named: name, fileExtension: ext) {
                return image
            }
        }
        return nil
    }()

    static func navigationImage(named name: String) -> NSImage? {
        guard let url = Bundle.main.url(forResource: name, withExtension: "png") else {
            return nil
        }
        guard let image = NSImage(contentsOf: url),
              let data = try? Data(contentsOf: url),
              let representation = NSBitmapImageRep(data: data),
              let cgImage = representation.cgImage else { return nil }
        // Original navigation resources include large transparent offsets.
        // Fit the visible artwork rather than scaling the padded canvas down.
        var minX = representation.pixelsWide
        var minY = representation.pixelsHigh
        var maxX = -1
        var maxY = -1
        for y in 0..<representation.pixelsHigh {
            for x in 0..<representation.pixelsWide {
                if (representation.colorAt(x: x, y: y)?.alphaComponent ?? 0) > 0 {
                    minX = min(minX, x); minY = min(minY, y)
                    maxX = max(maxX, x); maxY = max(maxY, y)
                }
            }
        }
        guard maxX >= minX, maxY >= minY else { return image }
        let bounds = CGRect(x: CGFloat(minX), y: CGFloat(minY),
                            width: CGFloat(maxX - minX + 1), height: CGFloat(maxY - minY + 1))
        guard let visible = cgImage.cropping(to: bounds) else { return image }
        return NSImage(cgImage: visible, size: bounds.size)
    }
}

private struct NavigationArtwork: View {
    let section: DashboardRootView.Section
    let isSelected: Bool

    var body: some View {
        if let image = ArtworkLoader.navigationImage(
            named: isSelected ? section.activeArtworkStem : section.artworkStem
        ) {
            Image(nsImage: image)
                .resizable()
                .scaledToFit()
        } else {
            Image(systemName: section.symbol)
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(Color.white.opacity(isSelected ? 0.96 : 0.72))
        }
    }
}

private struct AnimatedRGBEdge: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
    @Environment(\.scenePhase) private var scenePhase
    let cornerRadius: CGFloat
    let intensity: Double

    var body: some View {
        if reduceMotion || scenePhase != .active {
            RGBEdge(
                cornerRadius: cornerRadius,
                phase: 0,
                intensity: intensity,
                reduceTransparency: reduceTransparency
            )
        } else {
            TimelineView(.animation(minimumInterval: 1.0 / 30.0)) { context in
                let phase = context.date.timeIntervalSinceReferenceDate
                    .truncatingRemainder(dividingBy: 8) / 8
                RGBEdge(
                    cornerRadius: cornerRadius,
                    phase: phase,
                    intensity: intensity,
                    reduceTransparency: reduceTransparency
                )
            }
        }
    }
}

private struct RGBEdge: View {
    let cornerRadius: CGFloat
    let phase: Double
    let intensity: Double
    let reduceTransparency: Bool

    var body: some View {
        RoundedRectangle(cornerRadius: cornerRadius)
            .stroke(
                AngularGradient(
                    colors: [
                        DashboardPalette.cyan,
                        DashboardPalette.violet,
                        DashboardPalette.magenta,
                        DashboardPalette.cyan
                    ],
                    center: .center,
                    startAngle: .degrees(phase * 360),
                    endAngle: .degrees(phase * 360 + 360)
                ),
                lineWidth: 1
            )
            .shadow(color: DashboardPalette.cyan.opacity(reduceTransparency ? 0 : 0.62 * intensity), radius: 6)
            .shadow(color: DashboardPalette.magenta.opacity(reduceTransparency ? 0 : 0.48 * intensity), radius: 8)
    }
}

private struct DashboardBackdrop: View {
    let reduceTransparency: Bool

    var body: some View {
        ZStack {
            DashboardPalette.window
            if !reduceTransparency {
                ProductLineupBackground()
                    .opacity(0.34)
                    .overlay(
                        LinearGradient(
                            colors: [Color.black.opacity(0.32), Color.black.opacity(0.08), Color.black.opacity(0.42)],
                            startPoint: .top,
                            endPoint: .bottom
                        )
                    )
                RadialGradient(
                    colors: [DashboardPalette.cyan.opacity(0.075), .clear],
                    center: .topLeading,
                    startRadius: 10,
                    endRadius: 480
                )
                RadialGradient(
                    colors: [DashboardPalette.violet.opacity(0.065), .clear],
                    center: .topTrailing,
                    startRadius: 10,
                    endRadius: 520
                )
                RadialGradient(
                    colors: [DashboardPalette.magenta.opacity(0.040), .clear],
                    center: .bottomTrailing,
                    startRadius: 10,
                    endRadius: 500
                )
            }
        }
        .ignoresSafeArea()
        .allowsHitTesting(false)
        .accessibilityHidden(true)
    }
}

/// Decorative composition of bundled, original product cutouts. It is never
/// device state and remains hidden from assistive technologies.
private struct ProductLineupBackground: View {
    var body: some View {
        GeometryReader { proxy in
            let lineupHeight = min(max(proxy.size.height * 0.54, 320), 460)
            HStack(alignment: .bottom, spacing: -30) {
                LineupArtwork(name: "DroboMiniArtwork", fileExtension: "icns", height: lineupHeight * 0.66)
                LineupArtwork(name: "Drobo5DArtwork", fileExtension: "png", height: lineupHeight * 0.92)
                LineupArtwork(name: "DroboProArtwork", fileExtension: "icns", height: lineupHeight * 0.84)
                LineupArtwork(name: "DroboB800Artwork", fileExtension: "icns", height: lineupHeight * 0.78)
            }
            .frame(width: proxy.size.width * 1.06, height: lineupHeight, alignment: .bottom)
            .offset(x: -proxy.size.width * 0.03, y: proxy.size.height * 0.15)
            .blur(radius: 1.1)
            .saturation(0.72)
            .mask(
                LinearGradient(
                    colors: [.clear, .white.opacity(0.94), .white.opacity(0.86), .clear],
                    startPoint: .top,
                    endPoint: .bottom
                )
            )
        }
        .allowsHitTesting(false)
        .accessibilityHidden(true)
    }
}

private struct LineupArtwork: View {
    let name: String
    let fileExtension: String
    let height: CGFloat

    var body: some View {
        if let image = ArtworkLoader.image(named: name, fileExtension: fileExtension) {
            Image(nsImage: image)
                .resizable()
                .scaledToFit()
                .frame(height: height)
        }
    }
}

private extension View {
    @ViewBuilder
    func dashboardMainSurface(reduceTransparency: Bool) -> some View {
        if reduceTransparency {
            self
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background {
                    ZStack {
                        DashboardPalette.opaquePanel
                        DashboardPalette.darkBlend.opacity(0.24)
                    }
                }
        } else {
            self.frame(maxWidth: .infinity, maxHeight: .infinity).background(
                ZStack {
                    DashboardPalette.mainBlend
                    LinearGradient(
                        colors: [Color.black.opacity(0.06), Color.black.opacity(0.19)],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                }
            )
        }
    }

    @ViewBuilder
    func dashboardGlass(cornerRadius: CGFloat, tint: Color, reduceTransparency: Bool) -> some View {
        if reduceTransparency {
            self
                .background {
                    ZStack {
                        DashboardPalette.opaquePanel
                        DashboardPalette.darkBlend.opacity(0.18)
                    }
                }
                .clipShape(RoundedRectangle(cornerRadius: cornerRadius))
                .overlay(
                    RoundedRectangle(cornerRadius: cornerRadius)
                        .stroke(Color.white.opacity(0.18), lineWidth: 1)
                )
        } else {
            self
                .background(.ultraThinMaterial)
                .background(DashboardPalette.glassBlend)
                .glassEffect(.regular.tint(tint.opacity(0.24)), in: .rect(cornerRadius: cornerRadius))
                .clipShape(RoundedRectangle(cornerRadius: cornerRadius))
                .overlay(
                    RoundedRectangle(cornerRadius: cornerRadius)
                        .stroke(Color.white.opacity(0.16), lineWidth: 1)
                )
                .shadow(color: tint.opacity(0.10), radius: 14, y: 7)
        }
    }
}

private enum DashboardPalette {
    static let window = Color(red: 0.012, green: 0.015, blue: 0.022)
    static let opaquePanel = Color(red: 0.040, green: 0.045, blue: 0.060)
    static let cyan = Color(red: 0.20, green: 0.88, blue: 1.00)
    static let violet = Color(red: 0.58, green: 0.38, blue: 1.00)
    static let magenta = Color(red: 1.00, green: 0.24, blue: 0.76)
    static let info = Color(red: 0.32, green: 0.76, blue: 1.00)
    static let headerTint = Color(red: 0.10, green: 0.12, blue: 0.17)
    static let sidebarTint = Color(red: 0.055, green: 0.070, blue: 0.105)
    static let contentTint = Color(red: 0.040, green: 0.040, blue: 0.070)
    static let cardTint = Color(red: 0.075, green: 0.090, blue: 0.140)
    static let controlTint = Color(red: 0.10, green: 0.12, blue: 0.18)
    static let selectionTint = Color(red: 0.10, green: 0.15, blue: 0.28)
    static let mainBlend = LinearGradient(
        colors: [
            cyan.opacity(0.12),
            violet.opacity(0.075),
            magenta.opacity(0.065),
            Color.black.opacity(0.10)
        ],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )
    static let glassBlend = LinearGradient(
        colors: [
            cyan.opacity(0.075),
            violet.opacity(0.055),
            magenta.opacity(0.045)
        ],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )
    static let darkBlend = LinearGradient(
        colors: [
            Color(red: 0.025, green: 0.070, blue: 0.095),
            Color(red: 0.060, green: 0.035, blue: 0.100),
            Color(red: 0.085, green: 0.025, blue: 0.065)
        ],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )
}
