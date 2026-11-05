import Darwin
import Foundation

enum OpenDroboFSError: Error, CustomNSError {
    case posix(Int32), invalidName, foreignItem, unsupportedNode, capacityUnavailable(String)
    static var errorDomain: String { NSPOSIXErrorDomain }
    var errorCode: Int {
        switch self { case .posix(let code): return Int(code); case .invalidName: return Int(EINVAL); case .foreignItem: return Int(EXDEV); case .unsupportedNode: return Int(ENOTSUP); case .capacityUnavailable: return Int(EAGAIN) }
    }
    var errorUserInfo: [String: Any] { if case .capacityUnavailable(let text) = self { return [NSLocalizedDescriptionKey: text] }; return [:] }
}

func checkedName(_ data: Data) throws -> String {
    guard !data.isEmpty, data.count <= Int(NAME_MAX), !data.contains(0), let name = String(data: data, encoding: .utf8), name != ".", name != "..", !name.contains("/") else { throw OpenDroboFSError.invalidName }
    return name
}

enum NodeKind { case file, directory, symlink }
func fdKind(_ mode: mode_t) throws -> NodeKind {
    switch mode & S_IFMT { case S_IFREG: return .file; case S_IFDIR: return .directory; case S_IFLNK: return .symlink; default: throw OpenDroboFSError.unsupportedNode }
}

/// Descriptor-relative node. It never resolves a caller name by concatenating a path.
final class NodeFD {
    private(set) var fd: Int32
    let parentFD: Int32?
    let leafName: String?
    let kind: NodeKind
    let itemID: UInt64
    let parentItemID: UInt64
    let owner: UUID
    let rootDevice: dev_t
    private let device: dev_t
    private let inode: ino_t

    init(rootPath: String, owner: UUID) throws {
        let descriptor = open(rootPath, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK)
        guard descriptor >= 0 else { throw OpenDroboFSError.posix(errno) }
        var value = stat()
        guard fstat(descriptor, &value) == 0 else { let error = errno; close(descriptor); throw OpenDroboFSError.posix(error) }
        guard (value.st_mode & S_IFMT) == S_IFDIR else { close(descriptor); throw OpenDroboFSError.unsupportedNode }
        self.fd = descriptor; self.parentFD = nil; self.leafName = nil; self.kind = .directory
        self.itemID = 2; self.parentItemID = 1; self.owner = owner
        self.rootDevice = value.st_dev; self.device = value.st_dev; self.inode = value.st_ino
    }

    init(parent: NodeFD, nameData: Data) throws {
        guard parent.kind == .directory else { throw OpenDroboFSError.posix(ENOTDIR) }
        let name = try checkedName(nameData)
        var observed = stat()
        guard fstatat(parent.fd, name, &observed, AT_SYMLINK_NOFOLLOW) == 0 else { throw OpenDroboFSError.posix(errno) }
        let discoveredKind = try fdKind(observed.st_mode)
        guard observed.st_dev == parent.rootDevice else { throw OpenDroboFSError.posix(EXDEV) }
        let descriptor: Int32
        if discoveredKind == .symlink { descriptor = -1 }
        else {
            descriptor = openat(parent.fd, name, O_RDONLY | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK | (discoveredKind == .directory ? O_DIRECTORY : 0))
            guard descriptor >= 0 else { throw OpenDroboFSError.posix(errno) }
            var opened = stat()
            guard fstat(descriptor, &opened) == 0 else { let error = errno; close(descriptor); throw OpenDroboFSError.posix(error) }
            guard opened.st_dev == parent.rootDevice, opened.st_dev == observed.st_dev, opened.st_ino == observed.st_ino, (opened.st_mode & S_IFMT) == (observed.st_mode & S_IFMT) else { close(descriptor); throw OpenDroboFSError.posix(EAGAIN) }
        }
        let duplicatedParent = dup(parent.fd)
        guard duplicatedParent >= 0 else { if descriptor >= 0 { close(descriptor) }; throw OpenDroboFSError.posix(errno) }
        guard fcntl(duplicatedParent, F_SETFD, FD_CLOEXEC) == 0 else { let error = errno; close(duplicatedParent); if descriptor >= 0 { close(descriptor) }; throw OpenDroboFSError.posix(error) }
        self.fd = descriptor; self.parentFD = duplicatedParent; self.leafName = name; self.kind = discoveredKind
        self.itemID = NodeFD.stableID(observed.st_ino); self.parentItemID = parent.itemID; self.owner = parent.owner
        self.rootDevice = parent.rootDevice; self.device = observed.st_dev; self.inode = observed.st_ino
    }

    deinit { if fd >= 0 { close(fd) }; if let parentFD { close(parentFD) } }

    func statNoFollow() throws -> stat {
        var value = stat()
        if fd >= 0 { guard fstat(fd, &value) == 0 else { throw OpenDroboFSError.posix(errno) } }
        else if let parentFD, let leafName { guard fstatat(parentFD, leafName, &value, AT_SYMLINK_NOFOLLOW) == 0 else { throw OpenDroboFSError.posix(errno) } }
        else { throw OpenDroboFSError.posix(EBADF) }
        guard value.st_dev == device, value.st_ino == inode, value.st_dev == rootDevice, try fdKind(value.st_mode) == kind else { throw OpenDroboFSError.posix(EAGAIN) }
        return value
    }

    func verifyRootPath(_ path: String) throws {
        guard parentFD == nil else { return }
        let held = try statNoFollow()
        var pathValue = stat()
        guard lstat(path, &pathValue) == 0 else { throw OpenDroboFSError.posix(errno) }
        guard pathValue.st_dev == held.st_dev, pathValue.st_ino == held.st_ino, (pathValue.st_mode & S_IFMT) == S_IFDIR else { throw OpenDroboFSError.posix(EXDEV) }
        var heldFS = statfs(), pathFS = statfs()
        guard fstatfs(fd, &heldFS) == 0, statfs(path, &pathFS) == 0 else { throw OpenDroboFSError.posix(errno) }
        guard heldFS.f_fsid.val.0 == pathFS.f_fsid.val.0, heldFS.f_fsid.val.1 == pathFS.f_fsid.val.1 else { throw OpenDroboFSError.posix(EXDEV) }
    }
    func readLink() throws -> Data {
        guard kind == .symlink, let parentFD, let leafName else { throw OpenDroboFSError.posix(EINVAL) }
        _ = try statNoFollow()
        var bytes = [UInt8](repeating: 0, count: Int(PATH_MAX)); let count = readlinkat(parentFD, leafName, &bytes, bytes.count)
        guard count >= 0 else { throw OpenDroboFSError.posix(errno) }
        guard count < bytes.count else { throw OpenDroboFSError.posix(ENAMETOOLONG) }
        _ = try statNoFollow()
        let target = Data(bytes.prefix(Int(count)))
        guard !target.contains(0), let text = String(data: target, encoding: .utf8), !text.hasPrefix("/"), !text.split(separator: "/", omittingEmptySubsequences: false).contains("..") else { throw OpenDroboFSError.posix(EACCES) }
        return target
    }
    func read(offset: off_t, length: Int, into destination: UnsafeMutableRawBufferPointer) throws -> Int {
        guard kind == .file, fd >= 0, offset >= 0, length >= 0, length <= destination.count else { throw OpenDroboFSError.posix(EINVAL) }
        _ = try statNoFollow(); let result = pread(fd, destination.baseAddress, length, offset)
        guard result >= 0 else { throw OpenDroboFSError.posix(errno) }; return result
    }
    private static func stableID(_ inode: ino_t) -> UInt64 { let value = UInt64(inode) &* 0x9E3779B185EBCA87; return value <= 2 ? value + 3 : value }
}

struct DirectoryCursor {
    let position: UInt64
    static func start(cookie: UInt64, entryCount: Int, includeDots: Bool) throws -> DirectoryCursor {
        guard entryCount >= 0, cookie <= UInt64(Int.max) else { throw OpenDroboFSError.posix(EINVAL) }
        let position: UInt64
        if cookie == 0 { position = includeDots ? 0 : 2 }
        else if cookie == 1 { guard includeDots else { throw OpenDroboFSError.posix(EINVAL) }; position = 1 }
        else { position = cookie }
        guard position <= 2 + UInt64(entryCount) else { throw OpenDroboFSError.posix(EINVAL) }
        return DirectoryCursor(position: position)
    }
    var entryIndex: Int? { guard position >= 2, position - 2 <= UInt64(Int.max) else { return nil }; return Int(position - 2) }
    var nextCookie: UInt64 { position + 1 }
}

enum ResourceLifecyclePhase: Equatable { case idle, loading, loadReply, loaded, unloading, unloadReply }

/// Pure state machine. Callers hold their own lock; callbacks run only after a reply phase starts.
struct ResourceLifecycleGate {
    private(set) var phase: ResourceLifecyclePhase = .idle
    mutating func beginLoad() -> Bool { guard phase == .idle else { return false }; phase = .loading; return true }
    mutating func failLoad() { guard phase == .loading else { return }; phase = .idle }
    mutating func publishLoad() -> Bool { guard phase == .loading else { return false }; phase = .loadReply; return true }
    mutating func finishLoadReply() { guard phase == .loadReply else { return }; phase = .loaded }
    mutating func beginUnload() -> Bool { guard phase == .loaded else { return false }; phase = .unloading; return true }
    mutating func publishUnload() -> Bool { guard phase == .unloading else { return false }; phase = .unloadReply; return true }
    mutating func finishUnloadReply() { guard phase == .unloadReply else { return }; phase = .idle }
}

func directoryVerifier(seconds: Int64, nanoseconds: Int64) -> UInt64 {
    let secondsBits = UInt64(bitPattern: seconds)
    let nanosecondsBits = UInt64(bitPattern: nanoseconds)
    let mixed = (secondsBits &* 0x9E3779B185EBCA87) ^ (nanosecondsBits &* 0xC2B2AE3D27D4EB4F)
    return mixed == 0 ? 1 : mixed
}

struct ProtectedCapacity { let total: UInt64; let free: UInt64; let used: UInt64 }
struct ProtectedCapacitySource {
    let byteUnitsConfirmed: Bool
    let bind: (String) -> Result<UInt64, OpenDroboFSError>
    let capture: (UInt64) -> Result<(page: [UInt8], capturedAt: Int64), OpenDroboFSError>
    static let native = ProtectedCapacitySource(
        byteUnitsConfirmed: false,
        bind: { path in var identifier: UInt64 = 0; var count: UInt32 = 0; guard od_bind_volume(path, &identifier, &count) == 0, count == 1 else { return .failure(.capacityUnavailable("mounted path has no unique native Drobo binding")) }; return .success(identifier) },
        capture: { identifier in var page = [UInt8](repeating: 0, count: 1308); var length = 0; let capturedAt = Int64(Date().timeIntervalSince1970 * 1_000); guard od_capture_capacity(identifier, &page, page.count, &length) == 0, length > 0, length <= page.count else { return .failure(.capacityUnavailable("native vendor capacity capture failed")) }; return .success((Array(page.prefix(length)), capturedAt)) }
    )
}

/// Fresh vendor capacity only. A failed bind, identity check, capture, decode, or freshness check remains unavailable.
enum ProtectedCapacityProvider {
    static func bind(mountPath: String, source: ProtectedCapacitySource = .native) -> Result<UInt64, OpenDroboFSError> { source.bind(mountPath) }
    static func load(mountPath: String, registryID: UInt64, source: ProtectedCapacitySource = .native, nowMilliseconds: Int64? = nil) -> Result<ProtectedCapacity, OpenDroboFSError> {
        switch source.bind(mountPath) { case .success(let rebound) where rebound == registryID: break; case .success: return .failure(.capacityUnavailable("mounted path changed native Drobo binding")); case .failure(let error): return .failure(error) }
        switch source.capture(registryID) {
        case .failure(let error): return .failure(error)
        case .success(let capture):
            switch source.bind(mountPath) {
            case .success(let rebound) where rebound == registryID: break
            case .success: return .failure(.capacityUnavailable("mounted path changed native Drobo binding during capacity capture"))
            case .failure(let error): return .failure(error)
            }
            let now = nowMilliseconds ?? Int64(Date().timeIntervalSince1970 * 1_000); var capacity = ODCapacity()
            let confirmations = UInt32(OD_CONFIRM_SAME_DEVICE | OD_CONFIRM_SINGLE_VOLUME) | (source.byteUnitsConfirmed ? UInt32(OD_CONFIRM_BYTE_UNITS) : 0)
            guard od_capacity_decode(capture.page, capture.page.count, confirmations, capture.capturedAt, now, &capacity) == 0, od_capacity_validate(&capacity, now) == 0 else { return .failure(.capacityUnavailable("protected capacity unavailable: native byte units are unconfirmed")) }
            return .success(ProtectedCapacity(total: capacity.protected_total_bytes, free: capacity.protected_free_bytes, used: capacity.protected_used_bytes))
        }
    }
}
