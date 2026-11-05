import Foundation
import Combine

struct ReleaseInfo: Equatable, Sendable {
    let tag: String
    let pageURL: URL
}

enum ReleaseResult: Equatable, Sendable {
    case noReleases
    case current(String)
    case available(ReleaseInfo)
}

/// Interprets GitHub's stable-release API without downloading or executing code.
enum ReleasePolicy {
    static let repository = "hypler-dev/OpenDrobo"
    static let apiURL = URL(string: "https://api.github.com/repos/\(repository)/releases/latest")!
    static let releasesURL = URL(string: "https://github.com/\(repository)/releases")!

    struct Version: Comparable, Equatable {
        let major: Int
        let minor: Int
        let patch: Int

        init?(_ value: String) {
            let text = value.hasPrefix("v") ? String(value.dropFirst()) : value
            let parts = text.split(separator: ".", omittingEmptySubsequences: false)
            guard parts.count == 3,
                  parts.allSatisfy({ !$0.isEmpty && $0.utf8.allSatisfy { $0 >= 48 && $0 <= 57 } }),
                  let major = Int(parts[0]), let minor = Int(parts[1]), let patch = Int(parts[2])
            else { return nil }
            self.major = major; self.minor = minor; self.patch = patch
        }

        static func < (left: Self, right: Self) -> Bool {
            if left.major != right.major { return left.major < right.major }
            if left.minor != right.minor { return left.minor < right.minor }
            return left.patch < right.patch
        }
    }

    private struct APIRelease: Decodable {
        let tag_name: String
        let html_url: URL
        let draft: Bool
        let prerelease: Bool
    }

    static func evaluate(statusCode: Int, data: Data, installedVersion: String) throws -> ReleaseResult {
        if statusCode == 404 { return .noReleases }
        guard statusCode == 200 else {
            throw failure("GitHub returned HTTP \(statusCode). Try again later.")
        }
        guard data.count <= 2_097_152 else { throw failure("The release response is too large.") }
        let release = try JSONDecoder().decode(APIRelease.self, from: data)
        guard !release.draft, !release.prerelease else {
            throw failure("The release source did not return a stable published release.")
        }
        guard let current = Version(installedVersion), let latest = Version(release.tag_name) else {
            throw failure("The release version could not be compared with this build.")
        }
        let url = release.html_url
        guard url.scheme == "https", url.host == "github.com", url.port == nil,
              url.user == nil, url.password == nil, url.query == nil, url.fragment == nil,
              url.path == "/\(repository)/releases/tag/\(release.tag_name)" else {
            throw failure("The release link does not belong to the configured project.")
        }
        return latest > current
            ? .available(ReleaseInfo(tag: release.tag_name, pageURL: url))
            : .current(release.tag_name)
    }

    private static func failure(_ message: String) -> NSError {
        NSError(domain: "OpenDrobo.Releases", code: 1,
                userInfo: [NSLocalizedDescriptionKey: message])
    }
}

@MainActor
final class ReleaseUpdater: ObservableObject {
    static let releasesURL = ReleasePolicy.releasesURL
    @Published private(set) var message = "Updates come from hypler-dev/OpenDrobo on GitHub."
    @Published private(set) var isChecking = false
    @Published private(set) var availableRelease: ReleaseInfo?
    @Published var automaticChecks: Bool {
        didSet { UserDefaults.standard.set(automaticChecks, forKey: "automaticallyCheckUpdates") }
    }

    init() {
        automaticChecks = UserDefaults.standard.bool(forKey: "automaticallyCheckUpdates")
    }

    func check() async {
        guard !isChecking else { return }
        isChecking = true
        availableRelease = nil
        message = "Checking the latest published release…"
        defer { isChecking = false }
        do {
            var request = URLRequest(url: ReleasePolicy.apiURL, timeoutInterval: 20)
            request.setValue("application/vnd.github+json", forHTTPHeaderField: "Accept")
            request.setValue("2026-03-10", forHTTPHeaderField: "X-GitHub-Api-Version")
            request.setValue("OpenDrobo/0.1.0", forHTTPHeaderField: "User-Agent")
            let (data, response) = try await URLSession.shared.data(for: request)
            guard let response = response as? HTTPURLResponse,
                  response.url?.host == "api.github.com" else {
                throw URLError(.badServerResponse)
            }
            let current = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString")
                as? String ?? "0.1.0"
            switch try ReleasePolicy.evaluate(statusCode: response.statusCode, data: data,
                                              installedVersion: current) {
            case .noReleases: message = "No app releases have been published yet."
            case .current: message = "This build is up to date."
            case .available(let release):
                availableRelease = release
                message = "\(release.tag) is available from the project’s GitHub releases."
            }
        } catch {
            message = "Update check failed: \(error.localizedDescription)"
        }
    }
}
