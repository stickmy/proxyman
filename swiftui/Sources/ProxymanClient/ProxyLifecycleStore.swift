import Combine
import Foundation

@MainActor
final class ProxyLifecycleStore: ObservableObject {
    @Published private(set) var status = ProxyStatus(
        state: .disconnected,
        host: defaultProxyHost,
        port: defaultProxyPort
    )
    @Published var listenPortText = String(defaultProxyPort)
    @Published private(set) var lastError: String?
    @Published private(set) var isBusy = false

    private let coreClient: CoreClient

    init(coreClient: CoreClient) {
        self.coreClient = coreClient
    }

    var canEditEndpoint: Bool {
        !isBusy && status.state != .running && status.state != .starting && status.state != .stopping
    }

    var targetText: String {
        "\(status.host):\(statusPort)"
    }

    var hasValidListenPort: Bool {
        listenPort != nil
    }

    var listenPort: Int? {
        parsedListenPort()
    }

    var statusPort: Int {
        listenPort ?? status.port
    }

    func clearLastError() {
        lastError = nil
    }

    func refreshStatus() async {
        await perform {
            let nextStatus = try await coreClient.status()
            status = nextStatus
            if nextStatus.state == .running {
                listenPortText = String(nextStatus.port)
            } else {
                let preferredPort = parsedListenPort() ?? nextStatus.port
                let availablePort = try await coreClient.availablePort(
                    host: nextStatus.host,
                    port: preferredPort
                )
                status = ProxyStatus(
                    state: nextStatus.state,
                    host: nextStatus.host,
                    port: availablePort
                )
                listenPortText = String(availablePort)
            }
        }
    }

    func start() async {
        guard let port = parsedListenPort() else {
            lastError = "Port must be between 1 and 65535"
            return
        }

        let previousStatus = status
        await perform(onError: {
            self.status = previousStatus
        }) {
            status = .init(state: .starting, host: status.host, port: port)
            status = try await coreClient.startProxy(
                host: status.host,
                port: port,
                findAvailable: false
            )
            listenPortText = String(status.port)
        }
    }

    func stop() async {
        let previousStatus = status
        await perform(onError: {
            self.status = previousStatus
        }) {
            status = .init(state: .stopping, host: status.host, port: status.port)
            _ = try await coreClient.stopProxy()
            status = .init(
                state: .stopped,
                host: previousStatus.host,
                port: previousStatus.port
            )
        }
    }

    func apply(_ payload: ProxyStatusPayload) {
        status = ProxyStatus(
            state: payload.running ? .running : .stopped,
            host: payload.host,
            port: payload.port
        )
        listenPortText = String(payload.port)
    }

    private func perform(
        onError: (() -> Void)? = nil,
        _ operation: () async throws -> Void
    ) async {
        isBusy = true
        lastError = nil
        defer { isBusy = false }

        do {
            try await operation()
        } catch {
            onError?()
            lastError = error.localizedDescription
        }
    }

    private func parsedListenPort() -> Int? {
        let trimmed = listenPortText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let port = Int(trimmed), (1...65535).contains(port) else {
            return nil
        }
        return port
    }
}
