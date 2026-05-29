import Foundation

let defaultProxyHost = "127.0.0.1"
let defaultProxyPort = 9000

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

enum RuleEditorState: Equatable {
    case idle
    case saved
    case pending
    case validating
    case saving
    case invalid
    case failed(String)
}

enum CaptureReplayState: Equatable {
    case idle
    case running
    case succeeded(Int)
    case failed(String)
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
