import Darwin
import Foundation
import FSKit
import OSLog

private let fsLog = Logger(subsystem: "com.hypler.opendrobo.fskit", category: "fskit")
private func posix(_ code: Int32) -> NSError { NSError(domain: NSPOSIXErrorDomain, code: Int(code)) }

#if !OD_FSKIT_TEST
@main
struct OpenDroboFSExtension: UnaryFileSystemExtension {
    let fileSystem = OpenDroboFileSystem()
}
#endif

final class OpenDroboItem: FSItem { let node: NodeFD; init(_ node: NodeFD) { self.node = node } }

final class OpenDroboFileSystem: FSUnaryFileSystem, FSUnaryFileSystemOperations {
    private var resource: FSPathURLResource?
    private var lifecycle = ResourceLifecycleGate()
    private let lifecycleLock = NSRecursiveLock()
    func probeResource(resource: FSResource, replyHandler reply: @escaping (FSProbeResult?, (any Error)?) -> Void) {
        guard let path = resource as? FSPathURLResource, path.url.isFileURL else { reply(.notRecognized, nil); return }
        reply(.usableButLimited, nil)
    }
    func loadResource(resource: FSResource, options: FSTaskOptions, replyHandler reply: @escaping (FSVolume?, (any Error)?) -> Void) {
        lifecycleLock.lock()
        guard lifecycle.beginLoad() else { lifecycleLock.unlock(); reply(nil, posix(EBUSY)); return }
        lifecycleLock.unlock()
        guard let path = resource as? FSPathURLResource, path.url.isFileURL else { finishLoad(); reply(nil, posix(EINVAL)); return }
        guard path.url.startAccessingSecurityScopedResource() else { finishLoad(); reply(nil, posix(EACCES)); return }
        let force = options.taskOptions.contains { $0.contains("-f") }
        guard !force else { path.url.stopAccessingSecurityScopedResource(); finishLoad(); reply(nil, posix(ENOTSUP)); return }
        do {
            let volume = try OpenDroboVolume(rootPath: path.url.path)
            lifecycleLock.lock(); guard lifecycle.publishLoad() else { lifecycleLock.unlock(); path.url.stopAccessingSecurityScopedResource(); reply(nil, posix(EBUSY)); return }; self.resource = path; lifecycleLock.unlock()
            containerStatus = .ready; reply(volume, nil)
            lifecycleLock.lock(); lifecycle.finishLoadReply(); lifecycleLock.unlock()
        } catch { path.url.stopAccessingSecurityScopedResource(); finishLoad(); reply(nil, error) }
    }
    func unloadResource(resource: FSResource, options: FSTaskOptions, replyHandler reply: @escaping ((any Error)?) -> Void) {
        lifecycleLock.lock()
        guard let path = resource as? FSPathURLResource, let loaded = self.resource, path.url == loaded.url else { lifecycleLock.unlock(); reply(posix(EINVAL)); return }
        guard lifecycle.beginUnload() else { lifecycleLock.unlock(); reply(posix(EBUSY)); return }
        lifecycleLock.unlock()
        loaded.url.stopAccessingSecurityScopedResource()
        lifecycleLock.lock(); guard lifecycle.publishUnload() else { lifecycleLock.unlock(); reply(posix(EIO)); return }; self.resource = nil; lifecycleLock.unlock()
        containerStatus = .notReady(status: posix(ENODEV)); reply(nil)
        lifecycleLock.lock(); lifecycle.finishUnloadReply(); lifecycleLock.unlock()
    }
    private func finishLoad() { lifecycleLock.lock(); lifecycle.failLoad(); lifecycleLock.unlock() }
}

final class OpenDroboVolume: FSVolume, FSVolume.Operations, FSVolume.OpenCloseOperations, FSVolume.ReadWriteOperations {
    private let rootPath: String
    private let token: UUID
    private let rootNode: NodeFD
    private let registryID: UInt64
    private let capacitySource: ProtectedCapacitySource
    private var active = false

    convenience init(rootPath: String) throws { try self.init(rootPath: rootPath, capacitySource: .native) }
    init(rootPath: String, capacitySource: ProtectedCapacitySource) throws {
        let expected: UInt64
        switch ProtectedCapacityProvider.bind(mountPath: rootPath, source: capacitySource) { case .success(let value): expected = value; case .failure(let error): throw error }
        let token = UUID(); let root = try NodeFD(rootPath: rootPath, owner: token)
        try root.verifyRootPath(rootPath)
        switch ProtectedCapacityProvider.bind(mountPath: rootPath, source: capacitySource) {
        case .success(let rebound) where rebound == expected: break
        case .success: throw OpenDroboFSError.capacityUnavailable("mounted path changed native Drobo binding during initialization")
        case .failure(let error): throw error
        }
        let identifier = expected
        self.rootPath = rootPath; self.token = token; self.rootNode = root; self.registryID = identifier; self.capacitySource = capacitySource
        super.init(volumeID: FSVolume.Identifier(uuid: UUID()), volumeName: FSFileName(string: "OpenDrobo"))
    }

    var maximumLinkCount: Int { 1 }; var maximumNameLength: Int { Int(NAME_MAX) }
    var restrictsOwnershipChanges: Bool { true }; var truncatesLongNames: Bool { false }
    var supportedVolumeCapabilities: FSVolume.SupportedCapabilities {
        let value = FSVolume.SupportedCapabilities()
        value.supportsSymbolicLinks = true; value.supportsFastStatFS = false; value.supports64BitObjectIDs = true
        value.doesNotSupportSettingFilePermissions = true; value.supportsPersistentObjectIDs = false
        return value
    }
    var volumeStatistics: FSStatFSResult {
        let result = FSStatFSResult(fileSystemTypeName: "opendrobo"); result.blockSize = 4096; result.ioSize = 4096
        do {
            try rootNode.verifyRootPath(rootPath)
            let protectedCapacity = ProtectedCapacityProvider.load(mountPath: rootPath, registryID: registryID, source: capacitySource)
            try rootNode.verifyRootPath(rootPath)
            switch protectedCapacity {
            case .success(let capacity):
                guard capacity.total % 4096 == 0, capacity.used % 4096 == 0, capacity.free % 4096 == 0 else {
                    fsLog.error("protected capacity unavailable: byte counts do not match verified block geometry")
                    return result
                }
                result.totalBytes = capacity.total; result.availableBytes = capacity.free; result.freeBytes = capacity.free; result.usedBytes = capacity.used
                result.totalBlocks = capacity.total / 4096; result.availableBlocks = capacity.free / 4096; result.freeBlocks = capacity.free / 4096; result.usedBlocks = capacity.used / 4096
            case .failure(let error): fsLog.error("protected capacity unavailable: \(String(describing: error), privacy: .public)")
            }
        } catch { fsLog.error("protected capacity unavailable: root device changed") }
        return result
    }
    func mount(options: FSTaskOptions, replyHandler reply: @escaping ((any Error)?) -> Void) { reply(nil) }
    func unmount(replyHandler reply: @escaping () -> Void) { reply() }
    func synchronize(flags: FSSyncFlags, replyHandler reply: @escaping ((any Error)?) -> Void) { reply(nil) }
    func activate(options: FSTaskOptions, replyHandler reply: @escaping (FSItem?, (any Error)?) -> Void) { active = true; reply(OpenDroboItem(rootNode), nil) }
    func deactivate(options: FSDeactivateOptions, replyHandler reply: @escaping ((any Error)?) -> Void) { active = false; reply(nil) }

    func getAttributes(_ request: FSItem.GetAttributesRequest, of item: FSItem, replyHandler reply: @escaping (FSItem.Attributes?, (any Error)?) -> Void) {
        do { reply(try attributes(for: try checked(item)), nil) } catch { reply(nil, error) }
    }
    func setAttributes(_ request: FSItem.SetAttributesRequest, on item: FSItem, replyHandler reply: @escaping (FSItem.Attributes?, (any Error)?) -> Void) { reply(nil, posix(EROFS)) }
    func lookupItem(named name: FSFileName, inDirectory directory: FSItem, replyHandler reply: @escaping (FSItem?, FSFileName?, (any Error)?) -> Void) {
        do { let parent = try checked(directory); guard parent.kind == .directory else { throw OpenDroboFSError.posix(ENOTDIR) }; reply(OpenDroboItem(try NodeFD(parent: parent, nameData: name.data)), name, nil) }
        catch { reply(nil, nil, error) }
    }
    func reclaimItem(_ item: FSItem, replyHandler reply: @escaping ((any Error)?) -> Void) { reply(nil) }
    func readSymbolicLink(_ item: FSItem, replyHandler reply: @escaping (FSFileName?, (any Error)?) -> Void) { do { reply(FSFileName(data: try checked(item).readLink()), nil) } catch { reply(nil, error) } }
    func createItem(named name: FSFileName, type: FSItem.ItemType, inDirectory directory: FSItem, attributes: FSItem.SetAttributesRequest, replyHandler reply: @escaping (FSItem?, FSFileName?, (any Error)?) -> Void) { reply(nil, nil, posix(EROFS)) }
    func createSymbolicLink(named name: FSFileName, inDirectory directory: FSItem, attributes: FSItem.SetAttributesRequest, linkContents: FSFileName, replyHandler reply: @escaping (FSItem?, FSFileName?, (any Error)?) -> Void) { reply(nil, nil, posix(EROFS)) }
    func createLink(to item: FSItem, named name: FSFileName, inDirectory directory: FSItem, replyHandler reply: @escaping (FSFileName?, (any Error)?) -> Void) { reply(nil, posix(EROFS)) }
    func removeItem(_ item: FSItem, named name: FSFileName, fromDirectory directory: FSItem, replyHandler reply: @escaping ((any Error)?) -> Void) { reply(posix(EROFS)) }
    func renameItem(_ item: FSItem, inDirectory sourceDirectory: FSItem, named sourceName: FSFileName, to destinationName: FSFileName, inDirectory destinationDirectory: FSItem, overItem: FSItem?, replyHandler reply: @escaping (FSFileName?, (any Error)?) -> Void) { reply(nil, posix(EROFS)) }

    func enumerateDirectory(_ directory: FSItem, startingAt cookie: FSDirectoryCookie, verifier: FSDirectoryVerifier, attributes request: FSItem.GetAttributesRequest?, packer: FSDirectoryEntryPacker, replyHandler reply: @escaping (FSDirectoryVerifier, (any Error)?) -> Void) {
        do {
            let node = try checked(directory); guard node.kind == .directory, node.fd >= 0 else { throw OpenDroboFSError.posix(ENOTDIR) }
            let entries = try directoryEntries(node.fd).sorted { $0.name < $1.name }; let current = try verifierFor(node)
            if verifier != .initial && verifier != current { throw OpenDroboFSError.posix(EAGAIN) }
            var cursor = try DirectoryCursor.start(cookie: cookie.rawValue, entryCount: entries.count, includeDots: request == nil)
            while true {
                if cursor.position == 0 { guard packer.packEntry(name: FSFileName(string: "."), itemType: .directory, itemID: fsID(node.itemID), nextCookie: FSDirectoryCookie(rawValue: cursor.nextCookie), attributes: nil) else { break } }
                else if cursor.position == 1 { guard packer.packEntry(name: FSFileName(string: ".."), itemType: .directory, itemID: fsID(node.parentItemID), nextCookie: FSDirectoryCookie(rawValue: cursor.nextCookie), attributes: nil) else { break } }
                else if let index = cursor.entryIndex, index < entries.count {
                    let entry = entries[index]; let child = try NodeFD(parent: node, nameData: Data(entry.name.utf8)); let attributes = try request.map { _ in try self.attributes(for: child) }
                    guard packer.packEntry(name: FSFileName(string: entry.name), itemType: fsKind(child.kind), itemID: fsID(child.itemID), nextCookie: FSDirectoryCookie(rawValue: cursor.nextCookie), attributes: attributes) else { break }
                } else { break }
                cursor = try DirectoryCursor.start(cookie: cursor.nextCookie, entryCount: entries.count, includeDots: request == nil)
            }
            reply(current, nil)
        } catch { reply(.initial, error) }
    }
    func openItem(_ item: FSItem, modes: FSVolume.OpenModes, replyHandler reply: @escaping ((any Error)?) -> Void) { do { _ = try checked(item); reply(modes.contains(.write) ? posix(EROFS) : nil) } catch { reply(error) } }
    func closeItem(_ item: FSItem, modes: FSVolume.OpenModes, replyHandler reply: @escaping ((any Error)?) -> Void) { reply(nil) }
    func read(from item: FSItem, at offset: off_t, length: Int, into buffer: FSMutableFileDataBuffer, replyHandler reply: @escaping (Int, (any Error)?) -> Void) {
        do {
            let node = try checked(item); guard length >= 0, offset >= 0 else { throw OpenDroboFSError.posix(EINVAL) }
            let count = buffer.withUnsafeMutableBytes { raw -> Int in guard length <= raw.count else { return -Int(EINVAL) }; do { return try node.read(offset: offset, length: length, into: raw) } catch OpenDroboFSError.posix(let code) { return -Int(code) } catch { return -Int(EIO) } }
            reply(count < 0 ? 0 : count, count < 0 ? posix(Int32(-count)) : nil)
        } catch { reply(0, error) }
    }
    func write(contents: Data, to item: FSItem, at offset: off_t, replyHandler reply: @escaping (Int, (any Error)?) -> Void) { reply(0, posix(EROFS)) }

    private func checked(_ item: FSItem) throws -> NodeFD { guard active, let typed = item as? OpenDroboItem, typed.node.owner == token else { throw OpenDroboFSError.foreignItem }; return typed.node }
    private func attributes(for node: NodeFD) throws -> FSItem.Attributes {
        let value = try node.statNoFollow(); let result = FSItem.Attributes(); let writable = UInt32(S_IWUSR | S_IWGRP | S_IWOTH)
        result.type = fsKind(node.kind); result.mode = UInt32(value.st_mode) & ~writable; result.linkCount = node.kind == .file || node.kind == .symlink ? 1 : UInt32(value.st_nlink)
        result.uid = value.st_uid; result.gid = value.st_gid; result.size = UInt64(max(0, value.st_size)); result.allocSize = UInt64(max(0, value.st_blocks)) * 512
        result.fileID = fsID(node.itemID); result.parentID = fsID(node.parentItemID); result.accessTime = value.st_atimespec; result.modifyTime = value.st_mtimespec; result.changeTime = value.st_ctimespec; result.birthTime = value.st_birthtimespec; result.inhibitKernelOffloadedIO = true
        return result
    }
    private func fsKind(_ kind: NodeKind) -> FSItem.ItemType { switch kind { case .file: return .file; case .directory: return .directory; case .symlink: return .symlink } }
    private func fsID(_ value: UInt64) -> FSItem.Identifier { FSItem.Identifier(rawValue: value)! }
    private func verifierFor(_ node: NodeFD) throws -> FSDirectoryVerifier { let value = try node.statNoFollow(); return FSDirectoryVerifier(rawValue: directoryVerifier(seconds: Int64(value.st_mtimespec.tv_sec), nanoseconds: Int64(value.st_mtimespec.tv_nsec))) }
    private func directoryEntries(_ fd: Int32) throws -> [(name: String, kind: NodeKind)] {
        let independent = openat(fd, ".", O_RDONLY | O_DIRECTORY | O_CLOEXEC | O_NONBLOCK); guard independent >= 0, let stream = fdopendir(independent) else { if independent >= 0 { close(independent) }; throw OpenDroboFSError.posix(errno) }
        defer { closedir(stream) }; var output: [(String, NodeKind)] = []
        while true {
            errno = 0
            guard let entry = readdir(stream) else { if errno != 0 { throw OpenDroboFSError.posix(errno) }; break }
            let name = withUnsafePointer(to: entry.pointee.d_name) { $0.withMemoryRebound(to: CChar.self, capacity: Int(NAME_MAX) + 1) { String(cString: $0) } }
            if name == "." || name == ".." { continue }
            let valid = try checkedName(Data(name.utf8))
            var value = stat(); guard fstatat(fd, valid, &value, AT_SYMLINK_NOFOLLOW) == 0 else { throw OpenDroboFSError.posix(errno) }
            output.append((valid, try fdKind(value.st_mode)))
        }
        return output
    }
}
