import Foundation
import FSKit

// Offline test runner for a prototype only. No file, disk, network, mount, or
// extension operation occurs here.
private var checks = 0

private func require(_ condition: @autoclosure () -> Bool, _ name: String) {
    guard condition() else { fatalError("FAIL: \(name)") }
    checks += 1
    print("PASS: \(name)")
}

private let confirmed = CapacityConfirmation(
    byteUnitsConfirmed: true,
    sameDeviceConfirmed: true,
    singleVolumeConfirmed: true
)
private let capturedAt = Date(timeIntervalSinceReferenceDate: 10_000)
private let freshNow = capturedAt.addingTimeInterval(30)

private func wire(
    bodyLength: Int = CapacityRecord.legacyBodyLength,
    page: UInt8 = CapacityRecord.pageCode,
    subpage: UInt8 = CapacityRecord.subpageCode,
    free: UInt64 = 7_000_000_000_000,
    used: UInt64 = 5_000_000_000_000,
    total: UInt64 = 12_000_000_000_000,
    unprotected: UInt64 = 12_500_000_000_000,
    padding: Data = Data()
) -> Data {
    var result = Data([page, subpage, UInt8(bodyLength >> 8), UInt8(bodyLength & 0xff)])
    for value in [free, used, total, unprotected] {
        for shift in stride(from: 56, through: 0, by: -8) {
            result.append(UInt8((value >> UInt64(shift)) & 0xff))
        }
    }
    if bodyLength == CapacityRecord.extendedBodyLength {
        // Raw PT extension: present but deliberately not interpreted.
        result.append(contentsOf: Array(repeating: 0xa5, count: 24))
    }
    result.append(padding)
    return result
}

private func rejected(
    _ data: Data,
    confirmation: CapacityConfirmation = confirmed,
    capturedAt: Date = capturedAt,
    now: Date = freshNow
) -> Bool {
    do {
        _ = try CapacityRecord.parse(wire: data, confirmation: confirmation, capturedAt: capturedAt, now: now)
        return false
    } catch {
        return true
    }
}

let logicalCeiling = UInt64(64_000_000_000_000)
let legacy = wire()
let record = try CapacityRecord.parse(
    wire: legacy,
    confirmation: confirmed,
    capturedAt: capturedAt,
    now: freshNow
)
require(record.protectedTotalBytes == 12_000_000_000_000, "protected total stays actual 12 TB")
require(record.protectedTotalBytes != logicalCeiling, "64 TB logical fixture does not become real total")
require(record.protectedUsedBytes == 5_000_000_000_000, "protected used matches wire")
require(record.protectedFreeBytes == 7_000_000_000_000, "protected free matches wire")
require(record.rawPTExtension == nil, "legacy record has no PT extension")

for length in 0...3 {
    require(rejected(Data(legacy.prefix(length))), "header truncated at \(length)")
}
for length in 4...35 {
    require(rejected(Data(legacy.prefix(length))), "32-byte record truncated at \(length)")
}

let extended = wire(bodyLength: CapacityRecord.extendedBodyLength)
let extendedRecord = try CapacityRecord.parse(
    wire: extended,
    confirmation: confirmed,
    capturedAt: capturedAt,
    now: freshNow
)
require(extendedRecord.rawPTExtension == Data(repeating: 0xa5, count: 24), "56-byte record keeps raw PT extension")
require(rejected(Data(extended.prefix(36))), "56-byte record rejects 36 actual bytes")
for length in 36...59 {
    require(rejected(Data(extended.prefix(length))), "56-byte record truncated at \(length)")
}

require(rejected(wire(page: 0x79)), "wrong page rejects")
require(rejected(wire(subpage: 0x03)), "wrong subpage rejects")
require(rejected(wire(bodyLength: 28)), "unsupported 28-byte body rejects")
require(rejected(wire(bodyLength: 60)), "unsupported 60-byte body rejects")
require(rejected(wire(free: 0, used: 0, total: 0, unprotected: 0)), "zero total rejects")
require(rejected(wire(free: 8, used: 5, total: 12)), "free plus used mismatch rejects")
require(rejected(wire(free: 13, used: 0, total: 12)), "free above total rejects")
require(rejected(wire(free: UInt64.max, used: 1, total: UInt64.max, unprotected: UInt64.max)), "overflow-sized inconsistent values reject")
require(rejected(wire(unprotected: 11_999_999_999_999)), "unprotected total below protected rejects")
require(!rejected(wire(padding: Data([0xde, 0xad]))), "trailing allocation padding accepts")
require(rejected(legacy, capturedAt: capturedAt, now: capturedAt.addingTimeInterval(30.001)), "stale observation rejects")
require(rejected(legacy, capturedAt: capturedAt, now: capturedAt.addingTimeInterval(-0.001)), "future observation rejects")
require(rejected(legacy, confirmation: CapacityConfirmation(byteUnitsConfirmed: false, sameDeviceConfirmed: true, singleVolumeConfirmed: true)), "unconfirmed byte units reject")
require(rejected(legacy, confirmation: CapacityConfirmation(byteUnitsConfirmed: true, sameDeviceConfirmed: false, singleVolumeConfirmed: true)), "unconfirmed device rejects")
require(rejected(legacy, confirmation: CapacityConfirmation(byteUnitsConfirmed: true, sameDeviceConfirmed: true, singleVolumeConfirmed: false)), "shared pool ambiguity rejects")

var prefixed = Data([0xff])
prefixed.append(legacy)
let slicedRecord = try CapacityRecord.parse(
    wire: prefixed.dropFirst(), confirmation: confirmed, capturedAt: capturedAt, now: freshNow
)
require(slicedRecord == record, "nonzero-index Data slice decodes without out-of-bounds access")

private func rejectedStatistics(at now: Date) -> Bool {
    do {
        _ = try FinderCapacityPreview.makeStatistics(from: record, now: now)
        return false
    } catch { return true }
}
require(rejectedStatistics(at: capturedAt.addingTimeInterval(31)), "record expires before statistics construction")
require(rejectedStatistics(at: capturedAt.addingTimeInterval(-1)), "future statistics observation rejects")

let statistics = try FinderCapacityPreview.makeStatistics(from: record, now: freshNow)
require(statistics.fileSystemTypeName == "OpenDroboPreview", "preview type name matches")
require(statistics.blockSize == 4096 && statistics.ioSize == 4096, "preview block and IO size match")
require(statistics.totalBytes == record.protectedTotalBytes, "preview total uses protected bytes")
require(statistics.usedBytes == record.protectedUsedBytes, "preview used uses protected bytes")
require(statistics.freeBytes == record.protectedFreeBytes, "preview free uses protected bytes")
require(statistics.availableBytes == record.protectedFreeBytes, "preview available uses protected bytes")

print("PASS: \(checks) capacity prototype checks")
