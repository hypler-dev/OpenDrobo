import AppKit
import SwiftUI

/// Native app entry point. `--scan-json` is read-only diagnostics: it neither
/// opens a GUI nor bootstraps a driver, daemon, or upstream Drobo model.
@main
enum OpenDroboLauncher {
    static func main() {
        if CommandLine.arguments.contains("--scan-json") {
            do {
                let encoder = JSONEncoder()
                encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
                let result = try encoder.encode(LocalDiscovery.scan())
                print(String(decoding: result, as: UTF8.self))
            } catch {
                fputs("\(error.localizedDescription)\n", stderr)
                exit(1)
            }
            return
        }
        OpenDroboApp.main()
    }
}

struct OpenDroboApp: App {
    var body: some Scene {
        Window("OpenDrobo Dashboard", id: "main") {
            DashboardRootView()
        }
        .defaultSize(width: 1040, height: 740)
        .windowResizability(.contentMinSize)
        .commands {
            CommandGroup(replacing: .newItem) { }
        }
    }
}
