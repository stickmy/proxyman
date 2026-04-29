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
