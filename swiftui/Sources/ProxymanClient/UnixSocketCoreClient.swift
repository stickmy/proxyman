import Darwin
import Foundation

struct JsonRpcErrorPayload: Decodable {
    var code: Int
    var message: String
}

struct JsonRpcResponse<Result: Decodable>: Decodable {
    var jsonrpc: String
    var id: String?
    var result: Result?
    var error: JsonRpcErrorPayload?
}

enum SidecarResponseDecoder {
    static func decode<Result: Decodable>(_ type: Result.Type, from data: Data) throws -> Result {
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

enum JSONValue: Codable {
    case string(String)
    case int(Int)
    case double(Double)
    case bool(Bool)
    case object([String: JSONValue])
    case array([JSONValue])
    case null

    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() {
            self = .null
        } else if let value = try? container.decode(Bool.self) {
            self = .bool(value)
        } else if let value = try? container.decode(Int.self) {
            self = .int(value)
        } else if let value = try? container.decode(Double.self) {
            self = .double(value)
        } else if let value = try? container.decode(String.self) {
            self = .string(value)
        } else if let value = try? container.decode([String: JSONValue].self) {
            self = .object(value)
        } else if let value = try? container.decode([JSONValue].self) {
            self = .array(value)
        } else {
            throw DecodingError.dataCorruptedError(
                in: container,
                debugDescription: "Unsupported JSON value"
            )
        }
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .string(let value):
            try container.encode(value)
        case .int(let value):
            try container.encode(value)
        case .double(let value):
            try container.encode(value)
        case .bool(let value):
            try container.encode(value)
        case .object(let value):
            try container.encode(value)
        case .array(let value):
            try container.encode(value)
        case .null:
            try container.encodeNil()
        }
    }

    var foundationValue: Any {
        switch self {
        case .string(let value):
            value
        case .int(let value):
            value
        case .double(let value):
            value
        case .bool(let value):
            value
        case .object(let value):
            value.mapValues(\.foundationValue)
        case .array(let value):
            value.map(\.foundationValue)
        case .null:
            NSNull()
        }
    }
}

struct ProxyStatusPayload: Decodable {
    var apiVersion: Int
    var running: Bool
    var state: String
    var host: String
    var port: Int
}

struct AvailablePortPayload: Decodable {
    var apiVersion: Int
    var host: String
    var port: Int
}

struct SessionSearchPayload: Decodable {
    var apiVersion: Int
    var exchanges: [SessionExchangePayload]
}

struct SessionExchangePayload: Decodable {
    var exchangeId: String
    var method: String?
    var uri: String?
    var host: String?
    var status: Int?
    var requestTime: Int64?
    var responseTime: Int64?
    var requestBodyRef: String?
    var responseBodyRef: String?
}

struct SessionBodyPayload: Decodable {
    var apiVersion: Int
    var body: String
}

struct ClearSessionPayload: Decodable {
    var apiVersion: Int
    var cleared: Int
}

struct SessionHarExportPayload: Decodable {
    var apiVersion: Int
    var har: JSONValue
}

struct SessionHarImportPayload: Decodable {
    var apiVersion: Int
    var imported: Int
}

struct ReplayResponsePayload: Decodable {
    var apiVersion: Int
    var status: Int
    var headers: [String: String]
    var body: String
}

struct CaStatusPayload: Decodable {
    var apiVersion: Int
    var installed: Bool
}

struct CaInstallPayload: Decodable {
    var apiVersion: Int
    var installed: Bool
}

struct UpstreamTlsPayload: Decodable {
    var apiVersion: Int
    var ignoreVerification: Bool
}

struct SystemProxyPayload: Decodable {
    var apiVersion: Int
    var enabled: Bool
}

struct SystemProxyStatusPayload: Decodable {
    var apiVersion: Int
    var enabled: Bool
    var matchesRequested: Bool
    var services: [SystemProxyServicePayload]
}

struct SystemProxyServicePayload: Decodable {
    var service: String
    var web: SystemProxyStatePayload
    var secureWeb: SystemProxyStatePayload
    var bypassDomains: [String]
}

struct SystemProxyStatePayload: Decodable {
    var enabled: Bool
    var server: String?
    var port: String?
}

struct ListRulePacksPayload: Decodable {
    var apiVersion: Int
    var packs: [RulePackSummaryPayload]
}

struct RulePackSummaryPayload: Decodable {
    var packName: String
    var enabled: Bool
}

struct RulePackRulesPayload: Decodable {
    var apiVersion: Int
    var packName: String
    var enabled: Bool
    var content: String
    var evaluationOrder: [String]
}

struct ValidateRulesPayload: Decodable {
    var apiVersion: Int
    var valid: Bool
    var evaluationOrder: [String]
}

struct RulePackMutationPayload: Decodable {
    var apiVersion: Int
    var changed: Bool
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
        let result = try SidecarResponseDecoder.decode(AvailablePortPayload.self, from: data)
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

    func upstreamTlsStatus() async throws -> Bool {
        let response: UpstreamTlsPayload = try sendCommand(
            method: "upstreamTls.status",
            params: ["apiVersion": 1]
        )
        return response.ignoreVerification
    }

    func updateUpstreamTls(ignoreVerification: Bool) async throws -> Bool {
        let response: UpstreamTlsPayload = try sendCommand(
            method: "upstreamTls.update",
            params: [
                "apiVersion": 1,
                "ignoreVerification": ignoreVerification,
            ]
        )
        return response.ignoreVerification
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
        return try SidecarResponseDecoder.decode(Result.self, from: data)
    }
}

struct SidecarLaunch {
    var configuration: UnixSocketConfiguration
    var process: Process
}

enum UnixSocketError: Error, LocalizedError {
    case pathTooLong(String)
    case socketFailed(String)
    case connectFailed(String)
    case writeFailed(String)
    case readFailed(String)
    case invalidResponse
    case rpcError(String)

    var errorDescription: String? {
        switch self {
        case .pathTooLong(let path):
            "Unix socket path is too long: \(path)"
        case .socketFailed(let message),
             .connectFailed(let message),
             .writeFailed(let message),
             .readFailed(let message):
            message
        case .invalidResponse:
            "Sidecar returned an invalid response"
        case .rpcError(let message):
            message
        }
    }
}

final class UnixSocketConnection {
    let path: String

    init(path: String) {
        self.path = path
    }

    func send(jsonObject: [String: Any]) throws -> Data {
        let payload = try JSONSerialization.data(withJSONObject: jsonObject)
        var line = payload
        line.append(0x0a)

        let descriptor = socket(AF_UNIX, SOCK_STREAM, 0)
        if descriptor < 0 {
            throw UnixSocketError.socketFailed(Self.errnoMessage(prefix: "socket"))
        }
        defer { close(descriptor) }

        try connect(descriptor: descriptor)
        try writeAll(descriptor: descriptor, data: line)
        return try readLine(descriptor: descriptor)
    }

    func readLines(_ onLine: (Data) -> Bool) throws {
        let descriptor = socket(AF_UNIX, SOCK_STREAM, 0)
        if descriptor < 0 {
            throw UnixSocketError.socketFailed(Self.errnoMessage(prefix: "socket"))
        }
        defer { close(descriptor) }

        try connect(descriptor: descriptor)

        var output = Data()
        var byte = UInt8(0)

        while true {
            let result = Darwin.read(descriptor, &byte, 1)
            if result < 0 {
                throw UnixSocketError.readFailed(Self.errnoMessage(prefix: "read"))
            }
            if result == 0 {
                return
            }
            if byte == 0x0a {
                if !onLine(output) {
                    return
                }
                output.removeAll(keepingCapacity: true)
            } else {
                output.append(byte)
            }
        }
    }

    private func connect(descriptor: Int32) throws {
        var address = sockaddr_un()
        address.sun_family = sa_family_t(AF_UNIX)

        let pathBytes = Array(path.utf8) + [0]
        let capacity = MemoryLayout.size(ofValue: address.sun_path)
        if pathBytes.count > capacity {
            throw UnixSocketError.pathTooLong(path)
        }

        withUnsafeMutableBytes(of: &address.sun_path) { rawBuffer in
            rawBuffer.copyBytes(from: pathBytes)
        }

        let length = socklen_t(
            MemoryLayout<sockaddr_un>.size - capacity + pathBytes.count
        )
        let result = withUnsafePointer(to: &address) { pointer in
            pointer.withMemoryRebound(to: sockaddr.self, capacity: 1) { socketAddress in
                Darwin.connect(descriptor, socketAddress, length)
            }
        }

        if result != 0 {
            throw UnixSocketError.connectFailed(Self.errnoMessage(prefix: "connect"))
        }
    }

    private func writeAll(descriptor: Int32, data: Data) throws {
        try data.withUnsafeBytes { rawBuffer in
            guard let baseAddress = rawBuffer.baseAddress else { return }
            var written = 0
            while written < data.count {
                let result = Darwin.write(
                    descriptor,
                    baseAddress.advanced(by: written),
                    data.count - written
                )
                if result < 0 {
                    throw UnixSocketError.writeFailed(Self.errnoMessage(prefix: "write"))
                }
                written += result
            }
        }
    }

    private func readLine(descriptor: Int32) throws -> Data {
        var output = Data()
        var byte = UInt8(0)

        while true {
            let result = Darwin.read(descriptor, &byte, 1)
            if result < 0 {
                throw UnixSocketError.readFailed(Self.errnoMessage(prefix: "read"))
            }
            if result == 0 {
                break
            }
            if byte == 0x0a {
                return output
            }
            output.append(byte)
        }

        if output.isEmpty {
            throw UnixSocketError.invalidResponse
        }
        return output
    }

    private static func errnoMessage(prefix: String) -> String {
        String(cString: strerror(errno)).isEmpty
            ? "\(prefix) failed"
            : "\(prefix) failed: \(String(cString: strerror(errno)))"
    }
}

enum SidecarLaunchError: Error, LocalizedError {
    case executableMissing
    case socketDidNotAppear(String)

    var errorDescription: String? {
        switch self {
        case .executableMissing:
            "Bundled proxyman-sidecar executable is missing"
        case .socketDidNotAppear(let path):
            "Sidecar did not create command socket: \(path)"
        }
    }
}

final class SidecarLauncher {
    static func start() throws -> SidecarLaunch {
        let executable = try sidecarExecutableURL()
        let runtimeDirectory = FileManager.default.temporaryDirectory
            .appendingPathComponent("proxyman-swiftui-client", isDirectory: true)
        let commandSocket = runtimeDirectory.appendingPathComponent("command.sock")
        let eventSocket = runtimeDirectory.appendingPathComponent("event.sock")

        try FileManager.default.createDirectory(
            at: runtimeDirectory,
            withIntermediateDirectories: true
        )
        try? FileManager.default.removeItem(at: commandSocket)
        try? FileManager.default.removeItem(at: eventSocket)

        let process = Process()
        process.executableURL = executable
        process.arguments = [
            "--command-socket",
            commandSocket.path,
            "--event-socket",
            eventSocket.path,
        ]
        process.standardOutput = FileHandle.standardOutput
        process.standardError = FileHandle.standardError
        try process.run()

        for _ in 0..<50 {
            if FileManager.default.fileExists(atPath: commandSocket.path) {
                return SidecarLaunch(
                    configuration: UnixSocketConfiguration(
                        commandSocketPath: commandSocket.path,
                        eventSocketPath: eventSocket.path
                    ),
                    process: process
                )
            }
            Thread.sleep(forTimeInterval: 0.1)
        }

        if process.isRunning {
            process.terminate()
        }
        throw SidecarLaunchError.socketDidNotAppear(commandSocket.path)
    }

    private static func sidecarExecutableURL() throws -> URL {
        let environment = ProcessInfo.processInfo.environment
        if let path = environment["PROXYMAN_SIDECAR_PATH"], !path.isEmpty {
            return URL(fileURLWithPath: path)
        }

        if let url = Bundle.main.url(forResource: "proxyman-sidecar", withExtension: nil) {
            return url
        }

        throw SidecarLaunchError.executableMissing
    }
}
