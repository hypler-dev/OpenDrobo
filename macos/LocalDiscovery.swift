import Foundation
import IOKit

struct LocalDrobo: Codable, Identifiable, Sendable {
    let id: UInt64
    let name: String
    let layer: String
}

/// Reads registry metadata only. It never opens a device or sends a command.
enum LocalDiscovery {
    static func matchesPCI(vendor: UInt32, device: UInt32) -> Bool {
        vendor == 0x4452 && (device == 0x3844 || device == 0x5570)
    }

    static func scan() throws -> [LocalDrobo] {
        var devices: [LocalDrobo] = []
        for className in ["IOPCIDevice", "IOSCSIPeripheralDeviceNub"] {
            var iterator: io_iterator_t = 0
            let result = IOServiceGetMatchingServices(
                kIOMainPortDefault, IOServiceMatching(className), &iterator)
            guard result == KERN_SUCCESS else {
                throw NSError(domain: NSMachErrorDomain, code: Int(result),
                              userInfo: [NSLocalizedDescriptionKey:
                                "Could not read the \(className) registry (\(result))."])
            }
            defer { IOObjectRelease(iterator) }
            while case let entry = IOIteratorNext(iterator), entry != 0 {
                defer { IOObjectRelease(entry) }
                let name: String
                let layer: String
                if className == "IOPCIDevice" {
                    guard let vendor = number(entry, "vendor-id"),
                          let device = number(entry, "device-id"),
                          matchesPCI(vendor: vendor, device: device) else { continue }
                    name = String(format: "Drobo Thunderbolt PCI endpoint %04x:%04x",
                                  vendor, device)
                    layer = "PCI endpoint detected; storage and management are unverified"
                } else {
                    let vendor = string(entry, "Vendor Identification")
                    let product = string(entry, "Product Identification")
                    guard DroboIdentity.isDrobo(vendor: vendor, product: product) else { continue }
                    name = [vendor, product].filter { !$0.isEmpty }.joined(separator: " ")
                    layer = "SCSI device detected; management is unverified"
                }
                var id: UInt64 = 0
                IORegistryEntryGetRegistryEntryID(entry, &id)
                devices.append(LocalDrobo(id: id, name: name, layer: layer))
            }
        }
        return devices
    }

    private static func property(_ entry: io_registry_entry_t, _ key: String) -> Any? {
        IORegistryEntryCreateCFProperty(entry, key as CFString, kCFAllocatorDefault, 0)?
            .takeRetainedValue()
    }

    static func decodeNumber(_ value: Any?) -> UInt32? {
        if let value = value as? NSNumber { return value.uint32Value }
        guard let bytes = value as? Data, bytes.count >= 4 else { return nil }
        return bytes.prefix(4).enumerated().reduce(UInt32(0)) {
            $0 | UInt32($1.element) << ($1.offset * 8)
        }
    }

    private static func number(_ entry: io_registry_entry_t, _ key: String) -> UInt32? {
        decodeNumber(property(entry, key))
    }

    private static func string(_ entry: io_registry_entry_t, _ key: String) -> String {
        let value = property(entry, key)
        let text: String
        if let bytes = value as? Data {
            text = String(decoding: bytes.prefix { $0 != 0 }, as: UTF8.self)
        } else {
            text = value as? String ?? ""
        }
        return text.trimmingCharacters(in: .whitespacesAndNewlines)
    }
}
