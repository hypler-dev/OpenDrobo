import Foundation

/// Offline capacity evidence for a prototype only. This is not a mounted
/// filesystem, existing Finder mount fix, driver, live hardware reading, or shipped app feature.
public struct CapacityRecord: Equatable {
    public static let pageCode: UInt8 = 0x7a
    public static let subpageCode: UInt8 = 0x02
    public static let legacyBodyLength = 32
    public static let extendedBodyLength = 56

    public let protectedFreeBytes: UInt64
    public let protectedUsedBytes: UInt64
    public let protectedTotalBytes: UInt64
    public let unprotectedTotalBytes: UInt64
    /// Raw firmware PT extension bytes. Semantics are intentionally not interpreted.
    public let rawPTExtension: Data?
    public let capturedAt: Date

    /// Parses only an explicitly confirmed byte-valued capacity page.
    public static func parse(
        wire: Data,
        confirmation: CapacityConfirmation,
        capturedAt: Date,
        now: Date
    ) throws -> CapacityRecord {
        try confirmation.validate()
        try validateObservation(capturedAt: capturedAt, now: now)

        guard wire.count >= 4 else {
            throw CapacityRecordError.truncated(expectedAtLeast: 4, actual: wire.count)
        }
        // Bound the copy and rebase sliced Data without copying allocation padding.
        let page = Array(wire.prefix(4 + extendedBodyLength))
        guard page[0] == pageCode, page[1] == subpageCode else {
            throw CapacityRecordError.unsupportedPage(page: page[0], subpage: page[1])
        }

        let bodyLength = Int(page[2]) << 8 | Int(page[3])
        guard bodyLength == legacyBodyLength || bodyLength == extendedBodyLength else {
            throw CapacityRecordError.unsupportedBodyLength(bodyLength)
        }

        let requiredLength = 4 + bodyLength
        guard wire.count >= requiredLength else {
            throw CapacityRecordError.truncated(expectedAtLeast: requiredLength, actual: wire.count)
        }

        let protectedFreeBytes = try readBE64(page, at: 4, declaredLength: requiredLength)
        let protectedUsedBytes = try readBE64(page, at: 12, declaredLength: requiredLength)
        let protectedTotalBytes = try readBE64(page, at: 20, declaredLength: requiredLength)
        let unprotectedTotalBytes = try readBE64(page, at: 28, declaredLength: requiredLength)

        guard protectedTotalBytes != 0 else {
            throw CapacityRecordError.zeroOrNotReadyTotal
        }
        guard protectedFreeBytes <= protectedTotalBytes,
              protectedUsedBytes <= protectedTotalBytes,
              protectedTotalBytes - protectedFreeBytes == protectedUsedBytes else {
            throw CapacityRecordError.inconsistentProtectedCounts
        }
        guard unprotectedTotalBytes >= protectedTotalBytes else {
            throw CapacityRecordError.unprotectedTotalBelowProtectedTotal
        }

        let rawPTExtension: Data?
        if bodyLength == extendedBodyLength {
            rawPTExtension = Data(page[36..<60])
        } else {
            rawPTExtension = nil
        }

        return CapacityRecord(
            protectedFreeBytes: protectedFreeBytes,
            protectedUsedBytes: protectedUsedBytes,
            protectedTotalBytes: protectedTotalBytes,
            unprotectedTotalBytes: unprotectedTotalBytes,
            rawPTExtension: rawPTExtension,
            capturedAt: capturedAt
        )
    }

    static func validateObservation(capturedAt: Date, now: Date) throws {
        let age = now.timeIntervalSince(capturedAt)
        guard age >= 0 else { throw CapacityRecordError.futureObservation }
        guard age <= 30 else { throw CapacityRecordError.staleObservation }
    }

    private static func readBE64(_ wire: [UInt8], at offset: Int, declaredLength: Int) throws -> UInt64 {
        let end = offset + 8
        guard end <= declaredLength else {
            throw CapacityRecordError.truncated(expectedAtLeast: end, actual: wire.count)
        }
        var value: UInt64 = 0
        for byte in wire[offset..<end] {
            value = (value << 8) | UInt64(byte)
        }
        return value
    }
}

/// Required caller-supplied scope checks for capacity evidence.
public struct CapacityConfirmation: Equatable {
    public let byteUnitsConfirmed: Bool
    public let sameDeviceConfirmed: Bool
    public let singleVolumeConfirmed: Bool

    public init(
        byteUnitsConfirmed: Bool,
        sameDeviceConfirmed: Bool,
        singleVolumeConfirmed: Bool
    ) {
        self.byteUnitsConfirmed = byteUnitsConfirmed
        self.sameDeviceConfirmed = sameDeviceConfirmed
        self.singleVolumeConfirmed = singleVolumeConfirmed
    }

    fileprivate func validate() throws {
        guard byteUnitsConfirmed else { throw CapacityRecordError.byteUnitsUnconfirmed }
        guard sameDeviceConfirmed else { throw CapacityRecordError.deviceNotConfirmed }
        guard singleVolumeConfirmed else { throw CapacityRecordError.sharedPoolOrVolumeUnconfirmed }
    }
}

public enum CapacityRecordError: Error, Equatable {
    case byteUnitsUnconfirmed
    case deviceNotConfirmed
    case sharedPoolOrVolumeUnconfirmed
    case futureObservation
    case staleObservation
    case truncated(expectedAtLeast: Int, actual: Int)
    case unsupportedPage(page: UInt8, subpage: UInt8)
    case unsupportedBodyLength(Int)
    case zeroOrNotReadyTotal
    case inconsistentProtectedCounts
    case unprotectedTotalBelowProtectedTotal
}
