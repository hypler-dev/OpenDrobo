import Foundation

func require(_ condition: @autoclosure () -> Bool, _ message: String) {
    guard condition() else { fatalError(message) }
    print("PASS: \(message)")
}

require(LocalDiscovery.matchesPCI(vendor: 0x4452, device: 0x3844), "Original DMA PCI personality matches")
require(LocalDiscovery.matchesPCI(vendor: 0x4452, device: 0x5570), "Original PCI personality matches")
require(!LocalDiscovery.matchesPCI(vendor: 0x8086, device: 0x3844), "Other PCI vendors do not match")
require(!LocalDiscovery.matchesPCI(vendor: 0x4452, device: 0xffff), "Unknown Drobo PCI devices do not match")
require(LocalDiscovery.decodeNumber(Data([0x52, 0x44, 0, 0])) == 0x4452, "IOKit PCI vendor bytes decode little endian")
require(LocalDiscovery.decodeNumber(Data([0x70, 0x55, 0, 0])) == 0x5570, "IOKit PCI device bytes decode little endian")
require(LocalDiscovery.decodeNumber(Data([0x52, 0x44])) == nil, "Truncated PCI properties remain unknown")
require(LocalDiscovery.matchesSCSI(vendor: "Drobo", product: "5D - TB"), "Original Thunderbolt SCSI vendor matches")
require(LocalDiscovery.matchesSCSI(vendor: "  drObO\n", product: "8D - TB"), "SCSI vendor matching normalizes padding and case")
require(LocalDiscovery.matchesSCSI(vendor: "USB 3.0", product: " drobo s - gen 2 "), "Original legacy SCSI vendor-product pair matches")
require(!LocalDiscovery.matchesSCSI(vendor: "USB 3.0", product: "Other disk"), "Generic USB vendor alone does not identify a Drobo")
require(!LocalDiscovery.matchesSCSI(vendor: "Other vendor", product: "Drobo"), "Product branding alone does not establish vendor identity")
require(!LocalDiscovery.matchesSCSI(vendor: "TRUSTED", product: "USB Mass Storage"), "Generic legacy mass storage requires more identity evidence")
require(!LocalDiscovery.matchesSCSI(vendor: "", product: "Drobo"), "Missing SCSI vendor remains unidentified")
func releaseData(_ tag: String = "v0.2.0", url: String? = nil,
                 draft: Bool = false, prerelease: Bool = false) -> Data {
    try! JSONSerialization.data(withJSONObject: [
        "tag_name": tag,
        "html_url": url ?? "https://github.com/hypler-dev/OpenDrobo/releases/tag/\(tag)",
        "draft": draft, "prerelease": prerelease,
    ])
}

func rejected(_ data: Data, status: Int = 200) -> Bool {
    do {
        _ = try ReleasePolicy.evaluate(statusCode: status, data: data, installedVersion: "0.1.0")
        return false
    } catch { return true }
}

let update = try ReleasePolicy.evaluate(statusCode: 200, data: releaseData(), installedVersion: "0.1.0")
require(update == .available(ReleaseInfo(tag: "v0.2.0", pageURL: URL(string: "https://github.com/hypler-dev/OpenDrobo/releases/tag/v0.2.0")!)), "Latest release uses the configured source repository")
let emptyChannel = try ReleasePolicy.evaluate(statusCode: 404, data: Data(), installedVersion: "0.1.0")
require(emptyChannel == .noReleases, "An empty release channel has an explicit state")
let sameVersion = try ReleasePolicy.evaluate(statusCode: 200, data: releaseData("v0.1.0"), installedVersion: "0.1.0")
require(sameVersion == .current("v0.1.0"), "Equal versions do not propose an update")
require(ReleasePolicy.Version("0.10.0")! > ReleasePolicy.Version("0.2.0")!, "Release versions compare numerically")
require(rejected(releaseData(draft: true)), "Draft releases are rejected")
require(rejected(releaseData(prerelease: true)), "Prereleases are rejected")
require(rejected(releaseData("latest")), "Uncomparable tags are rejected")
require(rejected(releaseData(url: "https://example.com/update")), "Off-project update links are rejected")
require(rejected(releaseData(url: "https://github.com/other/project/releases/tag/v0.2.0")), "Other GitHub repositories are rejected")
require(rejected(Data("not JSON".utf8)), "Malformed update responses fail closed")
require(rejected(releaseData(), status: 429), "Rate limits remain a failure instead of claiming current")
