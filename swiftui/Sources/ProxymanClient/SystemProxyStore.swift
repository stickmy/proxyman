import Combine
import Foundation

@MainActor
final class SystemProxyStore: ObservableObject {
    @Published private(set) var status: SystemProxyStatus?
    @Published private(set) var lastError: String?
    @Published private(set) var isBusy = false

    private let coreClient: CoreClient

    init(coreClient: CoreClient) {
        self.coreClient = coreClient
    }

    func clearLastError() {
        lastError = nil
    }

    func refreshStatus(host: String, port: Int) async {
        await perform {
            status = try await coreClient.systemProxyStatus(host: host, port: port)
        }
    }

    func enable(host: String, port: Int) async {
        await perform {
            _ = try await coreClient.enableSystemProxy(port: port)
            status = try await coreClient.systemProxyStatus(host: host, port: port)
        }
    }

    func disable(host: String, statusPort: Int) async {
        await perform {
            _ = try await coreClient.disableSystemProxy()
            status = try await coreClient.systemProxyStatus(host: host, port: statusPort)
        }
    }

    private func perform(_ operation: () async throws -> Void) async {
        isBusy = true
        lastError = nil
        defer { isBusy = false }

        do {
            try await operation()
        } catch {
            lastError = error.localizedDescription
        }
    }
}
