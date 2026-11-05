import FSKit
import Foundation

/// FSKit statistics preview for a prototype only. This is not a mounted
/// filesystem, existing Finder mount fix, driver, live hardware reading, or shipped app feature.
/// It does not change disk geometry.
public enum FinderCapacityPreview {
    public static func makeStatistics(from record: CapacityRecord, now: Date) throws -> FSStatFSResult {
        // A record can expire between decoding and a volume-statistics request.
        try CapacityRecord.validateObservation(capturedAt: record.capturedAt, now: now)
        let result = FSStatFSResult(fileSystemTypeName: "OpenDroboPreview")
        result.blockSize = 4096
        result.ioSize = 4096
        result.totalBytes = record.protectedTotalBytes
        result.usedBytes = record.protectedUsedBytes
        result.freeBytes = record.protectedFreeBytes
        result.availableBytes = record.protectedFreeBytes
        return result
    }
}
