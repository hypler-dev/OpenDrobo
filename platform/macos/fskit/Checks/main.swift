import Darwin
import Foundation
import FSKit

@main
struct OpenDroboFSChecks {
    static func main() throws {
        let root = URL(fileURLWithPath: NSTemporaryDirectory()).appendingPathComponent("opendrobo-fskit-check-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let payload = Data("real passthrough bytes".utf8)
        try payload.write(to: root.appendingPathComponent("safe.txt"))
        try Data("inside".utf8).write(to: root.appendingPathComponent("inside.txt"))
        try FileManager.default.createSymbolicLink(atPath: root.appendingPathComponent("escape").path, withDestinationPath: "/etc/passwd")
        let owner = UUID(); let node = try NodeFD(rootPath: root.path, owner: owner)
        let file = try NodeFD(parent: node, nameData: Data("safe.txt".utf8))
        var destination = [UInt8](repeating: 0, count: payload.count)
        let count = try destination.withUnsafeMutableBytes { try file.read(offset: 0, length: $0.count, into: $0) }
        precondition(Data(destination.prefix(count)) == payload)
        for name in ["../escape", "/etc/passwd", "a/b", ".", "..", ""] { precondition((try? NodeFD(parent: node, nameData: Data(name.utf8))) == nil) }

        let link = try NodeFD(parent: node, nameData: Data("escape".utf8)); precondition(link.kind == .symlink)
        precondition((try? link.readLink()) == nil)
        try FileManager.default.removeItem(at: root.appendingPathComponent("escape"))
        try FileManager.default.createSymbolicLink(atPath: root.appendingPathComponent("escape").path, withDestinationPath: "../../escape")
        precondition((try? link.readLink()) == nil)
        try FileManager.default.removeItem(at: root.appendingPathComponent("escape"))
        try FileManager.default.createSymbolicLink(atPath: root.appendingPathComponent("escape").path, withDestinationPath: "inside.txt")
        let internalLink = try NodeFD(parent: node, nameData: Data("escape".utf8))
        let internalValue = try internalLink.readLink(); precondition(String(data: internalValue, encoding: .utf8) == "inside.txt")

        let originalInode = try file.statNoFollow().st_ino
        try FileManager.default.removeItem(at: root.appendingPathComponent("safe.txt"))
        let replacement = Data("replacement".utf8)
        try replacement.write(to: root.appendingPathComponent("safe.txt"))
        let heldInode = try file.statNoFollow().st_ino; precondition(heldInode == originalInode)
        var heldBytes = [UInt8](repeating: 0, count: payload.count)
        let heldCount = try heldBytes.withUnsafeMutableBytes { try file.read(offset: 0, length: $0.count, into: $0) }
        precondition(Data(heldBytes.prefix(heldCount)) == payload)
        let replacementFile = try NodeFD(parent: node, nameData: Data("safe.txt".utf8))
        let replacementInode = try replacementFile.statNoFollow().st_ino; precondition(replacementInode != originalInode)
        var replacementBytes = [UInt8](repeating: 0, count: replacement.count)
        let replacementCount = try replacementBytes.withUnsafeMutableBytes { try replacementFile.read(offset: 0, length: $0.count, into: $0) }
        precondition(Data(replacementBytes.prefix(replacementCount)) == replacement)
        precondition((try? file.read(offset: -1, length: 1, into: UnsafeMutableRawBufferPointer(start: nil, count: 0))) == nil)

        let dot = try DirectoryCursor.start(cookie: 0, entryCount: 2, includeDots: true)
        let dotDot = try DirectoryCursor.start(cookie: 1, entryCount: 2, includeDots: true)
        let first = try DirectoryCursor.start(cookie: 2, entryCount: 2, includeDots: true)
        let second = try DirectoryCursor.start(cookie: 3, entryCount: 2, includeDots: true)
        precondition(dot.position == 0 && dotDot.position == 1 && first.entryIndex == 0 && second.entryIndex == 1)
        precondition((try? DirectoryCursor.start(cookie: UInt64.max, entryCount: 2, includeDots: true)) == nil)
        let attributed = try DirectoryCursor.start(cookie: 0, entryCount: 2, includeDots: false); precondition(attributed.position == 2)
        precondition(directoryVerifier(seconds: -1, nanoseconds: 0) != 0)
        precondition(directoryVerifier(seconds: 0, nanoseconds: 0) != 0)
        precondition(directoryVerifier(seconds: Int64.min, nanoseconds: Int64.max) != 0)

        var lifecycle = ResourceLifecycleGate()
        precondition(lifecycle.beginLoad())
        precondition(lifecycle.publishLoad())
        precondition(!lifecycle.beginUnload()) // An unload concurrent with the load callback is rejected.
        precondition(!lifecycle.beginLoad())
        lifecycle.finishLoadReply()
        precondition(lifecycle.beginUnload())
        precondition(lifecycle.publishUnload())
        precondition(!lifecycle.beginLoad()) // An unload callback cannot race a new load.
        lifecycle.finishUnloadReply()
        precondition(lifecycle.beginLoad())
        lifecycle.failLoad()

        func capacityPage(free: UInt64, used: UInt64, total: UInt64, unprotected: UInt64) -> [UInt8] {
            var page = [UInt8(0x7a), 2, 0, 32]
            for value in [free, used, total, unprotected] { page += value.bigEndianBytes }
            return page
        }
        let free = UInt64(7 * 4096), used = UInt64(5 * 4096), total = free + used
        let page = capacityPage(free: free, used: used, total: total, unprotected: total + 4096)
        let byteUnitsUnconfirmed = ProtectedCapacitySource(byteUnitsConfirmed: false, bind: { _ in .success(41) }, capture: { _ in .success((page, Int64(Date().timeIntervalSince1970 * 1_000))) })
        if case .failure = ProtectedCapacityProvider.load(mountPath: root.path, registryID: 41, source: byteUnitsUnconfirmed) {} else { fatalError("stats unexpectedly bypassed byte confirmation") }
        let unavailableVolume = try OpenDroboVolume(rootPath: root.path, capacitySource: byteUnitsUnconfirmed)
        let unavailableStats = unavailableVolume.volumeStatistics
        precondition(unavailableStats.totalBytes == 0 && unavailableStats.availableBytes == 0 && unavailableStats.freeBytes == 0 && unavailableStats.usedBytes == 0)
        let confirmed = ProtectedCapacitySource(byteUnitsConfirmed: true, bind: { _ in .success(42) }, capture: { _ in .success((page, Int64(Date().timeIntervalSince1970 * 1_000))) })
        let volume = try OpenDroboVolume(rootPath: root.path, capacitySource: confirmed)
        let stats = volume.volumeStatistics
        precondition(stats.totalBytes == total && stats.availableBytes == free && stats.freeBytes == free && stats.usedBytes == used)
        precondition(stats.totalBlocks == total / 4096 && stats.availableBlocks == free / 4096 && stats.freeBlocks == free / 4096 && stats.usedBlocks == used / 4096)
        for (unalignedFree, unalignedUsed) in [(free + 1, used), (free + 1, used - 1), (free, used + 1)] {
            let unalignedPage = capacityPage(free: unalignedFree, used: unalignedUsed, total: unalignedFree + unalignedUsed, unprotected: unalignedFree + unalignedUsed + 4096)
            let source = ProtectedCapacitySource(byteUnitsConfirmed: true, bind: { _ in .success(43) }, capture: { _ in .success((unalignedPage, Int64(Date().timeIntervalSince1970 * 1_000))) })
            let unaligned = try OpenDroboVolume(rootPath: root.path, capacitySource: source).volumeStatistics
            precondition(unaligned.totalBytes == 0 && unaligned.availableBytes == 0 && unaligned.freeBytes == 0 && unaligned.usedBytes == 0)
            precondition(unaligned.totalBlocks == 0 && unaligned.availableBlocks == 0 && unaligned.freeBlocks == 0 && unaligned.usedBlocks == 0)
        }
        let movedRoot = root.deletingLastPathComponent().appendingPathComponent("opendrobo-fskit-held-root-\(UUID().uuidString)")
        var rootMoved = false
        defer {
            if rootMoved {
                try? FileManager.default.removeItem(at: root)
                try? FileManager.default.moveItem(at: movedRoot, to: root)
            }
        }
        try FileManager.default.moveItem(at: root, to: movedRoot)
        rootMoved = true
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: false)
        let remountedStats = volume.volumeStatistics
        precondition(remountedStats.totalBytes == 0 && remountedStats.availableBytes == 0 && remountedStats.freeBytes == 0 && remountedStats.usedBytes == 0)
        try FileManager.default.removeItem(at: root)
        try FileManager.default.moveItem(at: movedRoot, to: root)
        rootMoved = false
        final class CaptureRebind { var identifier: UInt64 = 61 }
        let captureRebind = CaptureRebind()
        let changedDuringCapture = ProtectedCapacitySource(byteUnitsConfirmed: true, bind: { _ in .success(captureRebind.identifier) }, capture: { _ in captureRebind.identifier = 62; return .success((page, Int64(Date().timeIntervalSince1970 * 1_000))) })
        if case .failure = ProtectedCapacityProvider.load(mountPath: root.path, registryID: 61, source: changedDuringCapture) {} else { fatalError("capture-time rebind published stale capacity") }

        let item = FSItem(); let request = FSItem.SetAttributesRequest()
        var attributeError: Error?; volume.setAttributes(request, on: item) { _, error in attributeError = error }
        var writeError: Error?; volume.write(contents: Data(), to: item, at: 0) { _, error in writeError = error }
        precondition((attributeError as NSError?)?.domain == NSPOSIXErrorDomain && (attributeError as NSError?)?.code == Int(EROFS))
        precondition((writeError as NSError?)?.domain == NSPOSIXErrorDomain && (writeError as NSError?)?.code == Int(EROFS))
        final class BindSequence { var values: [UInt64]; init(_ values: [UInt64]) { self.values = values }; func next() -> UInt64 { values.removeFirst() } }
        let rejectedSequence = BindSequence([7, 8])
        let changedBinding = ProtectedCapacitySource(byteUnitsConfirmed: false, bind: { _ in .success(rejectedSequence.next()) }, capture: { _ in .failure(.capacityUnavailable("unused")) })
        precondition((try? OpenDroboVolume(rootPath: root.path, capacitySource: changedBinding)) == nil)
        let acceptedSequence = BindSequence([7, 7])
        let stableBinding = ProtectedCapacitySource(byteUnitsConfirmed: false, bind: { _ in .success(acceptedSequence.next()) }, capture: { _ in .failure(.capacityUnavailable("unused")) })
        _ = try OpenDroboVolume(rootPath: root.path, capacitySource: stableBinding)
        print("PASS: read-only backend bytes, cursor resume, inode/symlink binding, mutation refusal, and zero unavailable stats")
    }
}

private extension FixedWidthInteger {
    var bigEndianBytes: [UInt8] { withUnsafeBytes(of: bigEndian, Array.init) }
}
