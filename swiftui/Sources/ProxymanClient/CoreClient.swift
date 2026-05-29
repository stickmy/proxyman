import Foundation

protocol CoreClient: Sendable {
    func status() async throws -> ProxyStatus
    func availablePort(host: String, port: Int) async throws -> Int
    func startProxy(host: String, port: Int, findAvailable: Bool) async throws -> ProxyStatus
    func stopProxy() async throws -> ProxyStatus
    func searchSessionExchanges() async throws -> [SessionExchangePayload]
    func loadSessionBody(bodyRef: String) async throws -> String
    func clearSession() async throws -> Int
    func exportSessionHar() async throws -> Data
    func importSessionHar(_ harData: Data) async throws -> Int
    func replaySession(exchangeID: String, edit: ReplayEdit) async throws -> ReplayResult
    func caStatus() async throws -> Bool
    func installCa() async throws -> Bool
    func upstreamTlsStatus() async throws -> Bool
    func updateUpstreamTls(ignoreVerification: Bool) async throws -> Bool
    func systemProxyStatus(host: String, port: Int) async throws -> SystemProxyStatus
    func enableSystemProxy(port: Int) async throws -> Bool
    func disableSystemProxy() async throws -> Bool
    func listRulePacks() async throws -> [RulePackSummary]
    func getRulePackRules(packName: String) async throws -> RulePackRules
    func validateRulePackRules(
        packName: String,
        enabled: Bool,
        content: String
    ) async throws -> RuleValidationResult
    func saveRulePackRules(packName: String, enabled: Bool, content: String) async throws -> Bool
    func addRulePack(packName: String, enabled: Bool) async throws -> Bool
    func removeRulePack(packName: String) async throws -> Bool
    func updateRulePackStatus(packName: String, enabled: Bool) async throws -> Bool
    func eventStream() -> AsyncStream<CoreEvent>
}

enum CoreClientError: Error, LocalizedError {
    case notConnected
    case invalidResponse

    var errorDescription: String? {
        switch self {
        case .notConnected:
            "Core sidecar is not connected"
        case .invalidResponse:
            "Core sidecar returned an invalid response"
        }
    }
}

final class MockCoreClient: CoreClient, @unchecked Sendable {
    private var running = false

    func status() async throws -> ProxyStatus {
        ProxyStatus(
            state: running ? .running : .stopped,
            host: defaultProxyHost,
            port: defaultProxyPort
        )
    }

    func availablePort(host: String, port: Int) async throws -> Int {
        port
    }

    func startProxy(host: String, port: Int, findAvailable: Bool) async throws -> ProxyStatus {
        running = true
        return ProxyStatus(state: .running, host: host, port: port)
    }

    func stopProxy() async throws -> ProxyStatus {
        running = false
        return ProxyStatus(state: .stopped, host: defaultProxyHost, port: defaultProxyPort)
    }

    func searchSessionExchanges() async throws -> [SessionExchangePayload] {
        []
    }

    func loadSessionBody(bodyRef: String) async throws -> String {
        ""
    }

    func clearSession() async throws -> Int {
        0
    }

    func exportSessionHar() async throws -> Data {
        Data(#"{"log":{"version":"1.2","creator":{"name":"Proxyman","version":"mock"},"entries":[]}}"#.utf8)
    }

    func importSessionHar(_ harData: Data) async throws -> Int {
        0
    }

    func replaySession(exchangeID: String, edit: ReplayEdit) async throws -> ReplayResult {
        ReplayResult(status: 200, headers: [:], body: "Mock replay response")
    }

    func caStatus() async throws -> Bool {
        false
    }

    func installCa() async throws -> Bool {
        true
    }

    func upstreamTlsStatus() async throws -> Bool {
        false
    }

    func updateUpstreamTls(ignoreVerification: Bool) async throws -> Bool {
        ignoreVerification
    }

    func systemProxyStatus(host: String, port: Int) async throws -> SystemProxyStatus {
        SystemProxyStatus(enabled: false, matchesRequested: false, services: [])
    }

    func enableSystemProxy(port: Int) async throws -> Bool {
        true
    }

    func disableSystemProxy() async throws -> Bool {
        false
    }

    func listRulePacks() async throws -> [RulePackSummary] {
        [
            RulePackSummary(packName: "default", enabled: true)
        ]
    }

    func getRulePackRules(packName: String) async throws -> RulePackRules {
        RulePackRules(
            packName: packName,
            enabled: true,
            content: "# mock\nredirect GET https://api.example.com/* https://mock.local\n",
            evaluationOrder: ["mock-rule"]
        )
    }

    func validateRulePackRules(
        packName: String,
        enabled: Bool,
        content: String
    ) async throws -> RuleValidationResult {
        RuleValidationResult(valid: true, evaluationOrder: ["mock-rule"])
    }

    func saveRulePackRules(packName: String, enabled: Bool, content: String) async throws -> Bool {
        true
    }

    func addRulePack(packName: String, enabled: Bool) async throws -> Bool {
        true
    }

    func removeRulePack(packName: String) async throws -> Bool {
        true
    }

    func updateRulePackStatus(packName: String, enabled: Bool) async throws -> Bool {
        true
    }

    func eventStream() -> AsyncStream<CoreEvent> {
        AsyncStream { continuation in
            continuation.finish()
        }
    }
}
