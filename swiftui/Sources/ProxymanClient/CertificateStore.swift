import Combine
import Foundation

@MainActor
final class CertificateStore: ObservableObject {
    @Published private(set) var caInstalled: Bool?
    @Published private(set) var ignoreUpstreamTlsVerification: Bool?
    @Published private(set) var lastError: String?
    @Published private(set) var isBusy = false

    private let coreClient: CoreClient

    init(coreClient: CoreClient) {
        self.coreClient = coreClient
    }

    func clearLastError() {
        lastError = nil
    }

    func refreshStatus() async {
        await perform {
            caInstalled = try await coreClient.caStatus()
        }
    }

    func install() async {
        await perform {
            caInstalled = try await coreClient.installCa()
        }
    }

    func refreshUpstreamTlsStatus() async {
        await perform {
            ignoreUpstreamTlsVerification = try await coreClient.upstreamTlsStatus()
        }
    }

    func setIgnoreUpstreamTlsVerification(_ enabled: Bool) async {
        await perform {
            ignoreUpstreamTlsVerification = try await coreClient.updateUpstreamTls(
                ignoreVerification: enabled
            )
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
