import Foundation

private let defaultProxyHost = "127.0.0.1"
private let defaultProxyPort = 9000

enum ProxyRunState: String {
    case disconnected = "Disconnected"
    case stopped = "Stopped"
    case starting = "Starting"
    case running = "Running"
    case stopping = "Stopping"
}

enum AppSection: String, CaseIterable, Hashable, Identifiable {
    case capture = "Capture"
    case rules = "Rules"
    case systemProxy = "System Proxy"
    case certificates = "Certificates"

    var id: String { rawValue }

    var systemImage: String {
        switch self {
        case .capture:
            "list.bullet.rectangle"
        case .rules:
            "slider.horizontal.3"
        case .systemProxy:
            "network"
        case .certificates:
            "checkmark.seal"
        }
    }
}

struct ProxyStatus: Equatable {
    var state: ProxyRunState
    var host: String
    var port: Int
}

struct ExchangeSummary: Identifiable, Hashable {
    var id: String
    var method: String
    var host: String
    var path: String
    var status: Int?
    var durationMillis: Int?
    var requestBytes: Int
    var responseBytes: Int
    var startedAt: Date
    var requestPreview: String
    var responsePreview: String
    var requestBodyRef: String?
    var responseBodyRef: String?

    var statusText: String {
        status.map(String.init) ?? "-"
    }

    var durationText: String {
        durationMillis.map { "\($0) ms" } ?? "-"
    }
}

struct ReplayEdit: Equatable {
    var method: String?
    var uri: String?
    var headers: [String: String]?
    var body: String?

    static let empty = ReplayEdit()

    var jsonObject: [String: Any] {
        var value: [String: Any] = [:]
        if let method {
            value["method"] = method
        }
        if let uri {
            value["uri"] = uri
        }
        if let headers {
            value["headers"] = headers
        }
        if let body {
            value["body"] = body
        }
        return value
    }
}

struct ReplayResult: Equatable {
    var status: Int
    var headers: [String: String]
    var body: String
}

struct RulePackSummary: Identifiable, Equatable {
    var packName: String
    var enabled: Bool

    var id: String { packName }
}

struct RulePackRules: Equatable {
    var packName: String
    var enabled: Bool
    var content: String
    var evaluationOrder: [String]
}

struct RuleValidationResult: Equatable {
    var valid: Bool
    var evaluationOrder: [String]
}

struct SystemProxyState: Equatable {
    var enabled: Bool
    var server: String?
    var port: String?

    var targetText: String {
        guard enabled, let server, let port else { return "-" }
        return "\(server):\(port)"
    }
}

struct SystemProxyServiceStatus: Identifiable, Equatable {
    var service: String
    var web: SystemProxyState
    var secureWeb: SystemProxyState
    var bypassDomains: [String]

    var id: String { service }
}

struct SystemProxyStatus: Equatable {
    var enabled: Bool
    var matchesRequested: Bool
    var services: [SystemProxyServiceStatus]
}

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

struct UnixSocketConfiguration: Equatable {
    var commandSocketPath: String
    var eventSocketPath: String
}

final class UnixSocketCoreClient: CoreClient, @unchecked Sendable {
    let configuration: UnixSocketConfiguration
    private let commandConnection: UnixSocketConnection
    private let sidecarProcess: Process?

    init(configuration: UnixSocketConfiguration, sidecarProcess: Process? = nil) {
        self.configuration = configuration
        self.sidecarProcess = sidecarProcess
        self.commandConnection = UnixSocketConnection(path: configuration.commandSocketPath)
    }

    deinit {
        if let sidecarProcess, sidecarProcess.isRunning {
            sidecarProcess.terminate()
        }
    }

    func status() async throws -> ProxyStatus {
        try await sendProxyCommand(method: "proxy.status", params: [:])
    }

    func availablePort(host: String, port: Int) async throws -> Int {
        let data = try commandConnection.send(
            jsonObject: [
                "jsonrpc": "2.0",
                "id": UUID().uuidString,
                "method": "proxy.availablePort",
                "params": [
                    "apiVersion": 1,
                    "host": host,
                    "port": port,
                ],
            ]
        )
        let response = try JSONDecoder().decode(JsonRpcResponse<AvailablePortPayload>.self, from: data)

        if let error = response.error {
            throw UnixSocketError.rpcError(error.message)
        }
        guard let result = response.result else {
            throw CoreClientError.invalidResponse
        }
        return result.port
    }

    func startProxy(host: String, port: Int, findAvailable: Bool) async throws -> ProxyStatus {
        try await sendProxyCommand(
            method: "proxy.start",
            params: [
                "apiVersion": 1,
                "host": host,
                "port": port,
                "findAvailable": findAvailable,
            ]
        )
    }

    func stopProxy() async throws -> ProxyStatus {
        try await sendProxyCommand(method: "proxy.stop", params: ["apiVersion": 1])
    }

    func searchSessionExchanges() async throws -> [SessionExchangePayload] {
        let response: SessionSearchPayload = try sendCommand(
            method: "sessions.search",
            params: [
                "apiVersion": 1,
                "filter": [:],
            ]
        )
        return response.exchanges
    }

    func loadSessionBody(bodyRef: String) async throws -> String {
        let response: SessionBodyPayload = try sendCommand(
            method: "sessions.loadBody",
            params: [
                "apiVersion": 1,
                "bodyRef": bodyRef,
            ]
        )
        return response.body
    }

    func clearSession() async throws -> Int {
        let response: ClearSessionPayload = try sendCommand(
            method: "sessions.clear",
            params: ["apiVersion": 1]
        )
        return response.cleared
    }

    func exportSessionHar() async throws -> Data {
        let response: SessionHarExportPayload = try sendCommand(
            method: "har.export",
            params: ["apiVersion": 1]
        )
        return try JSONSerialization.data(
            withJSONObject: response.har.foundationValue,
            options: [.prettyPrinted, .sortedKeys]
        )
    }

    func importSessionHar(_ harData: Data) async throws -> Int {
        let har = try JSONSerialization.jsonObject(with: harData)
        guard JSONSerialization.isValidJSONObject(har) else {
            throw CoreClientError.invalidResponse
        }
        let response: SessionHarImportPayload = try sendCommand(
            method: "har.import",
            params: [
                "apiVersion": 1,
                "har": har,
            ]
        )
        return response.imported
    }

    func replaySession(exchangeID: String, edit: ReplayEdit) async throws -> ReplayResult {
        let response: ReplayResponsePayload = try sendCommand(
            method: "replay.send",
            params: [
                "apiVersion": 1,
                "exchangeId": exchangeID,
                "edit": edit.jsonObject,
            ]
        )
        return ReplayResult(
            status: response.status,
            headers: response.headers,
            body: response.body
        )
    }

    func caStatus() async throws -> Bool {
        let response: CaStatusPayload = try sendCommand(
            method: "ca.status",
            params: ["apiVersion": 1]
        )
        return response.installed
    }

    func installCa() async throws -> Bool {
        let response: CaInstallPayload = try sendCommand(
            method: "ca.install",
            params: ["apiVersion": 1]
        )
        return response.installed
    }

    func systemProxyStatus(host: String, port: Int) async throws -> SystemProxyStatus {
        let response: SystemProxyStatusPayload = try sendCommand(
            method: "systemProxy.status",
            params: [
                "apiVersion": 1,
                "host": host,
                "port": port,
            ]
        )
        return SystemProxyStatus(
            enabled: response.enabled,
            matchesRequested: response.matchesRequested,
            services: response.services.map { service in
                SystemProxyServiceStatus(
                    service: service.service,
                    web: SystemProxyState(
                        enabled: service.web.enabled,
                        server: service.web.server,
                        port: service.web.port
                    ),
                    secureWeb: SystemProxyState(
                        enabled: service.secureWeb.enabled,
                        server: service.secureWeb.server,
                        port: service.secureWeb.port
                    ),
                    bypassDomains: service.bypassDomains
                )
            }
        )
    }

    func enableSystemProxy(port: Int) async throws -> Bool {
        let response: SystemProxyPayload = try sendCommand(
            method: "systemProxy.enable",
            params: [
                "apiVersion": 1,
                "port": port,
            ]
        )
        return response.enabled
    }

    func disableSystemProxy() async throws -> Bool {
        let response: SystemProxyPayload = try sendCommand(
            method: "systemProxy.disable",
            params: ["apiVersion": 1]
        )
        return response.enabled
    }

    func listRulePacks() async throws -> [RulePackSummary] {
        let response: ListRulePacksPayload = try sendCommand(
            method: "rules.listPacks",
            params: ["apiVersion": 1]
        )
        return response.packs.map {
            RulePackSummary(packName: $0.packName, enabled: $0.enabled)
        }
    }

    func getRulePackRules(packName: String) async throws -> RulePackRules {
        let response: RulePackRulesPayload = try sendCommand(
            method: "rules.getPackRules",
            params: [
                "apiVersion": 1,
                "packName": packName,
            ]
        )
        return RulePackRules(
            packName: response.packName,
            enabled: response.enabled,
            content: response.content,
            evaluationOrder: response.evaluationOrder
        )
    }

    func validateRulePackRules(
        packName: String,
        enabled: Bool,
        content: String
    ) async throws -> RuleValidationResult {
        let response: ValidateRulesPayload = try sendCommand(
            method: "rules.validate",
            params: [
                "apiVersion": 1,
                "packName": packName,
                "enabled": enabled,
                "content": content,
            ]
        )
        return RuleValidationResult(
            valid: response.valid,
            evaluationOrder: response.evaluationOrder
        )
    }

    func saveRulePackRules(packName: String, enabled: Bool, content: String) async throws -> Bool {
        let response: RulePackMutationPayload = try sendCommand(
            method: "rules.savePackRules",
            params: [
                "apiVersion": 1,
                "packName": packName,
                "enabled": enabled,
                "content": content,
            ]
        )
        return response.changed
    }

    func addRulePack(packName: String, enabled: Bool) async throws -> Bool {
        let response: RulePackMutationPayload = try sendCommand(
            method: "rules.addPack",
            params: [
                "apiVersion": 1,
                "packName": packName,
                "enabled": enabled,
            ]
        )
        return response.changed
    }

    func removeRulePack(packName: String) async throws -> Bool {
        let response: RulePackMutationPayload = try sendCommand(
            method: "rules.removePack",
            params: [
                "apiVersion": 1,
                "packName": packName,
            ]
        )
        return response.changed
    }

    func updateRulePackStatus(packName: String, enabled: Bool) async throws -> Bool {
        let response: RulePackMutationPayload = try sendCommand(
            method: "rules.updatePackStatus",
            params: [
                "apiVersion": 1,
                "packName": packName,
                "enabled": enabled,
            ]
        )
        return response.changed
    }

    func eventStream() -> AsyncStream<CoreEvent> {
        let eventSocketPath = configuration.eventSocketPath
        return AsyncStream { continuation in
            guard !eventSocketPath.isEmpty else {
                continuation.finish()
                return
            }

            let task = Task.detached(priority: .background) {
                let connection = UnixSocketConnection(path: eventSocketPath)
                do {
                    try connection.readLines { data in
                        guard !Task.isCancelled else { return false }
                        guard !data.isEmpty else { return true }

                        if let event = try? JSONDecoder().decode(CoreEvent.self, from: data) {
                            continuation.yield(event)
                        }
                        return true
                    }
                } catch {
                    continuation.finish()
                    return
                }

                continuation.finish()
            }

            continuation.onTermination = { _ in
                task.cancel()
            }
        }
    }

    private func sendProxyCommand(method: String, params: [String: Any]) async throws -> ProxyStatus {
        let result: ProxyStatusPayload = try sendCommand(method: method, params: params)
        return ProxyStatus(
            state: result.running ? .running : .stopped,
            host: result.host,
            port: result.port
        )
    }

    private func sendCommand<Result: Decodable>(method: String, params: [String: Any]) throws -> Result {
        let data = try commandConnection.send(
            jsonObject: [
                "jsonrpc": "2.0",
                "id": UUID().uuidString,
                "method": method,
                "params": params,
            ]
        )
        let response = try JSONDecoder().decode(JsonRpcResponse<Result>.self, from: data)

        if let error = response.error {
            throw UnixSocketError.rpcError(error.message)
        }
        guard let result = response.result else {
            throw CoreClientError.invalidResponse
        }

        return result
    }
}

@MainActor
final class AppModel: ObservableObject {
    @Published private(set) var proxyStatus = ProxyStatus(
        state: .disconnected,
        host: defaultProxyHost,
        port: defaultProxyPort
    )
    @Published var selectedSection: AppSection = .capture
    @Published private(set) var exchanges: [ExchangeSummary] = []
    @Published var selectedExchangeID: ExchangeSummary.ID?
    @Published var listenPortText = String(defaultProxyPort)
    @Published private(set) var lastError: String?
    @Published private(set) var isBusy = false
    @Published private(set) var rulePacks: [RulePackSummary] = []
    @Published var selectedRulePackName: String?
    @Published var ruleEditorContent = "" {
        didSet {
            guard !isLoadingRuleEditor else { return }
            ruleValidation = nil
        }
    }
    @Published var ruleEditorEnabled = true {
        didSet {
            guard !isLoadingRuleEditor else { return }
            ruleValidation = nil
        }
    }
    @Published var newRulePackName = ""
    @Published private(set) var ruleValidation: RuleValidationResult?
    @Published private(set) var caInstalled: Bool?
    @Published private(set) var systemProxyStatus: SystemProxyStatus?

    private let coreClient: CoreClient
    private var eventTask: Task<Void, Never>?
    private var ruleEditorOriginalContent = ""
    private var ruleEditorOriginalEnabled = true
    private var isLoadingRuleEditor = false

    init(coreClient: CoreClient) {
        self.coreClient = coreClient
    }

    var selectedExchange: ExchangeSummary? {
        exchanges.first { $0.id == selectedExchangeID }
    }

    var canEditEndpoint: Bool {
        !isBusy && proxyStatus.state != .running && proxyStatus.state != .starting && proxyStatus.state != .stopping
    }

    var selectedRulePack: RulePackSummary? {
        rulePacks.first { $0.packName == selectedRulePackName }
    }

    var hasSelectedRulePack: Bool {
        selectedRulePackName != nil
    }

    var isRuleEditorDirty: Bool {
        ruleEditorContent != ruleEditorOriginalContent || ruleEditorEnabled != ruleEditorOriginalEnabled
    }

    var canAddRulePack: Bool {
        !trimmedNewRulePackName().isEmpty && !isBusy
    }

    var canSaveRulePack: Bool {
        hasSelectedRulePack && isRuleEditorDirty && !isBusy
    }

    var systemProxyTargetText: String {
        let port = parsedListenPort() ?? proxyStatus.port
        return "\(proxyStatus.host):\(port)"
    }

    var canEnableSystemProxy: Bool {
        parsedListenPort() != nil && !isBusy
    }

    var canDisableSystemProxy: Bool {
        !isBusy
    }

    func refreshStatus() async {
        await perform {
            let status = try await coreClient.status()
            proxyStatus = status
            if status.state == .running {
                listenPortText = String(status.port)
            } else {
                let preferredPort = parsedListenPort() ?? status.port
                let availablePort = try await coreClient.availablePort(
                    host: status.host,
                    port: preferredPort
                )
                proxyStatus = ProxyStatus(
                    state: status.state,
                    host: status.host,
                    port: availablePort
                )
                listenPortText = String(availablePort)
            }
        }
    }

    func refreshSessionSummaries() async {
        do {
            let summaries = try await coreClient.searchSessionExchanges()
            mergeSessionSummaries(summaries)
        } catch {
            lastError = error.localizedDescription
        }
    }

    func startEventStream() {
        guard eventTask == nil else { return }

        eventTask = Task { [weak self] in
            guard let self else { return }
            for await event in coreClient.eventStream() {
                apply(event)
            }
        }
    }

    func startProxy() async {
        guard let port = parsedListenPort() else {
            lastError = "Port must be between 1 and 65535"
            return
        }

        await perform {
            proxyStatus = .init(state: .starting, host: proxyStatus.host, port: port)
            proxyStatus = try await coreClient.startProxy(
                host: proxyStatus.host,
                port: port,
                findAvailable: false
            )
            listenPortText = String(proxyStatus.port)
        }
    }

    func stopProxy() async {
        let previousStatus = proxyStatus
        await perform {
            proxyStatus = .init(state: .stopping, host: proxyStatus.host, port: proxyStatus.port)
            _ = try await coreClient.stopProxy()
            proxyStatus = .init(
                state: .stopped,
                host: previousStatus.host,
                port: previousStatus.port
            )
        }
    }

    func clearSession() async {
        await perform {
            _ = try await coreClient.clearSession()
            exchanges.removeAll()
            selectedExchangeID = nil
        }
    }

    func selectSection(_ section: AppSection) {
        selectedSection = section
    }

    func refreshRulePacks() async {
        await perform {
            let packs = try await coreClient.listRulePacks()
            rulePacks = sortedRulePacks(packs)
            if selectedRulePackName == nil || !rulePacks.contains(where: { $0.packName == selectedRulePackName }) {
                selectedRulePackName = rulePacks.first?.packName
            }
        }

        if let nextRulePackName = self.selectedRulePackName {
            await loadRulePack(named: nextRulePackName)
        } else {
            resetRuleEditor()
        }
    }

    func selectRulePack(_ packName: String) async {
        guard selectedRulePackName != packName || ruleEditorContent.isEmpty else { return }
        await loadRulePack(named: packName)
    }

    func validateSelectedRulePack() async {
        guard let selectedRulePackName else { return }

        await perform {
            ruleValidation = try await coreClient.validateRulePackRules(
                packName: selectedRulePackName,
                enabled: ruleEditorEnabled,
                content: ruleEditorContent
            )
        }
    }

    func saveSelectedRulePack() async {
        guard let selectedRulePackName else { return }

        await perform {
            let validation = try await coreClient.validateRulePackRules(
                packName: selectedRulePackName,
                enabled: ruleEditorEnabled,
                content: ruleEditorContent
            )
            ruleValidation = validation
            guard validation.valid else { return }

            _ = try await coreClient.saveRulePackRules(
                packName: selectedRulePackName,
                enabled: ruleEditorEnabled,
                content: ruleEditorContent
            )
            ruleEditorOriginalContent = ruleEditorContent
            ruleEditorOriginalEnabled = ruleEditorEnabled
            rulePacks = sortedRulePacks(try await coreClient.listRulePacks())
        }
    }

    func addRulePack() async {
        let packName = trimmedNewRulePackName()
        guard !packName.isEmpty else { return }

        await perform {
            _ = try await coreClient.addRulePack(packName: packName, enabled: true)
            newRulePackName = ""
            rulePacks = sortedRulePacks(try await coreClient.listRulePacks())
        }
        await loadRulePack(named: packName)
    }

    func removeSelectedRulePack() async {
        guard let selectedRulePackName else { return }

        await perform {
            _ = try await coreClient.removeRulePack(packName: selectedRulePackName)
            rulePacks = sortedRulePacks(try await coreClient.listRulePacks())
            self.selectedRulePackName = rulePacks.first?.packName
        }

        if let nextRulePackName = self.selectedRulePackName {
            await loadRulePack(named: nextRulePackName)
        } else {
            resetRuleEditor()
        }
    }

    func setRuleEditorEnabled(_ enabled: Bool) {
        ruleEditorEnabled = enabled
    }

    func refreshCaStatus() async {
        await perform {
            caInstalled = try await coreClient.caStatus()
        }
    }

    func installCa() async {
        await perform {
            caInstalled = try await coreClient.installCa()
        }
    }

    func refreshSystemProxyStatus() async {
        await perform {
            systemProxyStatus = try await coreClient.systemProxyStatus(
                host: proxyStatus.host,
                port: systemProxyStatusPort()
            )
        }
    }

    func enableSystemProxy() async {
        guard let port = parsedListenPort() else {
            lastError = "Port must be between 1 and 65535"
            return
        }

        await perform {
            _ = try await coreClient.enableSystemProxy(port: port)
            systemProxyStatus = try await coreClient.systemProxyStatus(
                host: proxyStatus.host,
                port: port
            )
        }
    }

    func disableSystemProxy() async {
        await perform {
            _ = try await coreClient.disableSystemProxy()
            systemProxyStatus = try await coreClient.systemProxyStatus(
                host: proxyStatus.host,
                port: systemProxyStatusPort()
            )
        }
    }

    func loadRulePack(named packName: String) async {
        await perform {
            let rules = try await coreClient.getRulePackRules(packName: packName)
            isLoadingRuleEditor = true
            defer { isLoadingRuleEditor = false }

            selectedRulePackName = rules.packName
            ruleEditorContent = rules.content
            ruleEditorEnabled = rules.enabled
            ruleEditorOriginalContent = rules.content
            ruleEditorOriginalEnabled = rules.enabled
            ruleValidation = nil

            if let index = rulePacks.firstIndex(where: { $0.packName == rules.packName }) {
                rulePacks[index].enabled = rules.enabled
            }
        }
    }

    func loadSelectedExchangeBodies() async {
        guard let selectedExchangeID else { return }

        do {
            let summaries = try await coreClient.searchSessionExchanges()
            mergeSessionSummaries(summaries)

            guard let index = exchanges.firstIndex(where: { $0.id == selectedExchangeID }) else {
                return
            }
            let requestBodyRef = exchanges[index].requestBodyRef
            let responseBodyRef = exchanges[index].responseBodyRef

            if let requestBodyRef {
                let body = try await coreClient.loadSessionBody(bodyRef: requestBodyRef)
                if let currentIndex = exchanges.firstIndex(where: { $0.id == selectedExchangeID }) {
                    exchanges[currentIndex].requestPreview = body
                    exchanges[currentIndex].requestBytes = body.utf8.count
                }
            }

            if let responseBodyRef {
                let body = try await coreClient.loadSessionBody(bodyRef: responseBodyRef)
                if let currentIndex = exchanges.firstIndex(where: { $0.id == selectedExchangeID }) {
                    exchanges[currentIndex].responsePreview = body
                    exchanges[currentIndex].responseBytes = body.utf8.count
                }
            }
        } catch {
            lastError = error.localizedDescription
        }
    }

    func replaySelectedExchange() async {
        guard let selectedExchangeID else { return }

        await perform {
            let result = try await coreClient.replaySession(
                exchangeID: selectedExchangeID,
                edit: .empty
            )
            if let index = exchanges.firstIndex(where: { $0.id == selectedExchangeID }) {
                exchanges[index].status = result.status
                exchanges[index].responsePreview = result.body
                exchanges[index].responseBytes = result.body.utf8.count
            }
        }
    }

    private func apply(_ event: CoreEvent) {
        switch event.payload {
        case .proxyEvent(let envelope):
            apply(envelope.event)
        case .proxyStatus(let payload):
            proxyStatus = ProxyStatus(
                state: payload.running ? .running : .stopped,
                host: payload.host,
                port: payload.port
            )
            listenPortText = String(payload.port)
        case nil:
            break
        }
    }

    private func apply(_ event: ProxyEvent) {
        switch event {
        case .exchangeStarted(let payload):
            updateExchange(
                id: payload.exchangeId,
                timestamp: payload.timestamp,
                uri: payload.uri,
                method: payload.method
            ) { exchange in
                let endpoint = Self.endpoint(from: payload.uri)
                exchange.method = payload.method
                exchange.host = endpoint.host
                exchange.path = endpoint.path
                exchange.startedAt = Self.date(fromMilliseconds: payload.timestamp)
                exchange.requestPreview = "\(payload.method) \(endpoint.path)"
            }
        case .requestHead(let payload):
            updateExchange(
                id: payload.exchangeId,
                timestamp: payload.timestamp,
                uri: payload.uri,
                method: payload.method
            ) { exchange in
                let endpoint = Self.endpoint(from: payload.uri)
                exchange.method = payload.method
                exchange.host = endpoint.host
                exchange.path = endpoint.path
                exchange.requestPreview = Self.headPreview(
                    title: "\(payload.method) \(endpoint.path) \(payload.version)",
                    headers: payload.headers,
                    body: payload.capturedBody
                )
                if let body = payload.capturedBody {
                    exchange.requestBytes = body.size
                }
            }
        case .requestBodyChunk(let payload):
            updateExchange(id: payload.exchangeId, timestamp: payload.timestamp, uri: payload.uri) { exchange in
                exchange.requestBytes += payload.byteLen
                exchange.requestPreview = Self.appendingPreview(payload.preview, to: exchange.requestPreview)
            }
        case .requestFinished(let payload):
            updateExchange(id: payload.exchangeId, timestamp: payload.timestamp) { exchange in
                if let body = payload.capturedBody {
                    exchange.requestBytes = body.size
                    if exchange.requestPreview.isEmpty {
                        exchange.requestPreview = body.preview
                    }
                }
            }
        case .responseHead(let payload):
            updateExchange(id: payload.exchangeId, timestamp: payload.timestamp, uri: payload.uri) { exchange in
                exchange.status = payload.status
                exchange.responsePreview = Self.headPreview(
                    title: "HTTP \(payload.version) \(payload.status)",
                    headers: payload.headers,
                    body: payload.capturedBody
                )
                if let body = payload.capturedBody {
                    exchange.responseBytes = body.size
                }
            }
        case .responseBodyChunk(let payload):
            updateExchange(id: payload.exchangeId, timestamp: payload.timestamp, uri: payload.uri) { exchange in
                exchange.responseBytes += payload.byteLen
                exchange.responsePreview = Self.appendingPreview(payload.preview, to: exchange.responsePreview)
            }
        case .responseFinished(let payload):
            updateExchange(id: payload.exchangeId, timestamp: payload.timestamp) { exchange in
                exchange.status = payload.status
                if let body = payload.capturedBody {
                    exchange.responseBytes = body.size
                    if exchange.responsePreview.isEmpty {
                        exchange.responsePreview = body.preview
                    }
                }
            }
        case .exchangeFinished(let payload):
            updateExchange(id: payload.exchangeId, timestamp: payload.timestamp) { exchange in
                exchange.status = payload.status ?? exchange.status
                exchange.durationMillis = Self.durationMillis(
                    from: exchange.startedAt,
                    toMilliseconds: payload.timestamp
                )
            }
        case .exchangeError(let payload):
            updateExchange(
                id: payload.exchangeId,
                timestamp: payload.timestamp,
                uri: payload.uri
            ) { exchange in
                exchange.responsePreview = "\(payload.phase): \(payload.message)"
            }
        case .sseEvent(let payload):
            updateExchange(id: payload.exchangeId, timestamp: payload.timestamp, uri: payload.uri) { exchange in
                let label = payload.event ?? "message"
                exchange.responsePreview = Self.appendingPreview("event: \(label)\ndata: \(payload.data)", to: exchange.responsePreview)
            }
        case .webSocketMessage(let payload):
            updateExchange(id: payload.exchangeId, timestamp: payload.timestamp, uri: payload.uri) { exchange in
                let message = "\(payload.direction) \(payload.opcode): \(payload.payloadPreview)"
                exchange.responsePreview = Self.appendingPreview(message, to: exchange.responsePreview)
                exchange.responseBytes += payload.payloadLen
            }
        case .unsupported:
            break
        }
    }

    private func updateExchange(
        id: String,
        timestamp: Int64,
        uri: String? = nil,
        method: String = "-",
        update: (inout ExchangeSummary) -> Void
    ) {
        if let index = exchanges.firstIndex(where: { $0.id == id }) {
            update(&exchanges[index])
            return
        }

        let endpoint = Self.endpoint(from: uri ?? "")
        var exchange = ExchangeSummary(
            id: id,
            method: method,
            host: endpoint.host,
            path: endpoint.path,
            status: nil,
            durationMillis: nil,
            requestBytes: 0,
            responseBytes: 0,
            startedAt: Self.date(fromMilliseconds: timestamp),
            requestPreview: "",
            responsePreview: "",
            requestBodyRef: nil,
            responseBodyRef: nil
        )
        update(&exchange)
        exchanges.insert(exchange, at: 0)
        if selectedExchangeID == nil {
            selectedExchangeID = id
        }
    }

    private func mergeSessionSummaries(_ summaries: [SessionExchangePayload]) {
        for payload in summaries.reversed() {
            let summary = Self.exchangeSummary(from: payload)
            if let index = exchanges.firstIndex(where: { $0.id == payload.exchangeId }) {
                exchanges[index].method = summary.method
                exchanges[index].host = summary.host
                exchanges[index].path = summary.path
                exchanges[index].status = summary.status
                exchanges[index].durationMillis = summary.durationMillis ?? exchanges[index].durationMillis
                exchanges[index].startedAt = summary.startedAt
                exchanges[index].requestBodyRef = summary.requestBodyRef
                exchanges[index].responseBodyRef = summary.responseBodyRef
            } else {
                exchanges.insert(summary, at: 0)
            }
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

    private func resetRuleEditor() {
        isLoadingRuleEditor = true
        defer { isLoadingRuleEditor = false }

        selectedRulePackName = nil
        ruleEditorContent = ""
        ruleEditorEnabled = true
        ruleEditorOriginalContent = ""
        ruleEditorOriginalEnabled = true
        ruleValidation = nil
    }

    private func parsedListenPort() -> Int? {
        let trimmed = listenPortText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let port = Int(trimmed), (1...65535).contains(port) else {
            return nil
        }
        return port
    }

    private func systemProxyStatusPort() -> Int {
        parsedListenPort() ?? proxyStatus.port
    }

    private func trimmedNewRulePackName() -> String {
        newRulePackName.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private func sortedRulePacks(_ packs: [RulePackSummary]) -> [RulePackSummary] {
        packs.sorted { left, right in
            left.packName.localizedStandardCompare(right.packName) == .orderedAscending
        }
    }

    private static func endpoint(from uri: String) -> (host: String, path: String) {
        guard let components = URLComponents(string: uri), let host = components.host else {
            return ("-", uri.isEmpty ? "-" : uri)
        }

        let displayHost = components.port.map { "\(host):\($0)" } ?? host
        var path = components.path.isEmpty ? "/" : components.path
        if let query = components.query, !query.isEmpty {
            path += "?\(query)"
        }
        return (displayHost, path)
    }

    private static func exchangeSummary(from payload: SessionExchangePayload) -> ExchangeSummary {
        let uri = payload.uri ?? ""
        let endpoint = Self.endpoint(from: uri)
        let requestTime = payload.requestTime ?? Int64(Date().timeIntervalSince1970 * 1000)
        let duration = payload.requestTime.flatMap { requestTime in
            payload.responseTime.map { max(0, Int($0 - requestTime)) }
        }

        return ExchangeSummary(
            id: payload.exchangeId,
            method: payload.method ?? "-",
            host: payload.host ?? endpoint.host,
            path: endpoint.path,
            status: payload.status,
            durationMillis: duration,
            requestBytes: 0,
            responseBytes: 0,
            startedAt: Self.date(fromMilliseconds: requestTime),
            requestPreview: "",
            responsePreview: "",
            requestBodyRef: payload.requestBodyRef,
            responseBodyRef: payload.responseBodyRef
        )
    }

    private static func date(fromMilliseconds milliseconds: Int64) -> Date {
        Date(timeIntervalSince1970: TimeInterval(milliseconds) / 1000)
    }

    private static func durationMillis(from startDate: Date, toMilliseconds endMilliseconds: Int64) -> Int {
        max(0, Int(TimeInterval(endMilliseconds) - startDate.timeIntervalSince1970 * 1000))
    }

    private static func headPreview(
        title: String,
        headers: [HeaderEntryPayload],
        body: CapturedBodyPayload?
    ) -> String {
        var lines = [title]
        lines.append(contentsOf: headers.map { "\($0.name): \($0.value)" })
        if let body, !body.preview.isEmpty {
            lines.append("")
            lines.append(body.preview)
        }
        return lines.joined(separator: "\n")
    }

    private static func appendingPreview(_ preview: String, to existing: String) -> String {
        guard !preview.isEmpty else { return existing }
        let combined = existing.isEmpty ? preview : "\(existing)\n\(preview)"
        let maxCharacters = 16_000
        if combined.count <= maxCharacters {
            return combined
        }
        return String(combined.suffix(maxCharacters))
    }
}
