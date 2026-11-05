import Foundation

/// Synthetic ESA records, kept separate from all hardware discovery results.
enum PreviewFixture {
    static var snapshot: DroboSnapshot { DroboSnapshot.decode(records) }

    static var records: [UInt8: Data] {
        let total: UInt64 = 6_000_000_000_000
        let used: UInt64 = 2_400_000_000_000
        let capacity = be64(total - used) + be64(used) + be64(total)
        var slots: [UInt8] = [5]
        for index in 1...5 {
            var entry = [UInt8](repeating: 0, count: 72)
            entry.replaceSubrange(3..<11, with: be64(2_000_000_000_000))
            let model = Array("Sample disk \(index)".utf8)
            entry.replaceSubrange(20..<(20 + model.count), with: model)
            slots += entry
        }
        var system = [UInt8](repeating: 0, count: 64)
        system.replaceSubrange(6..<18, with: Array("Sample Drobo".utf8))
        return [0x02: page(0x02, capacity), 0x03: page(0x03, slots),
                0x05: page(0x05, system)]
    }

    private static func be64(_ value: UInt64) -> [UInt8] {
        (0..<8).reversed().map { UInt8((value >> ($0 * 8)) & 0xff) }
    }

    private static func page(_ subpage: UInt8, _ payload: [UInt8]) -> Data {
        var bytes = [0x7a, subpage, UInt8(payload.count >> 8), UInt8(payload.count & 0xff)]
        bytes += payload
        bytes += [UInt8](repeating: 0, count: max(0, 1308 - bytes.count))
        return Data(bytes)
    }
}
