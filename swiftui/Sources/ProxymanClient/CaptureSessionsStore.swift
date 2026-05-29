import Combine
import Foundation

@MainActor
final class CaptureSessionsStore: ObservableObject {
    @Published private(set) var exchanges: [ExchangeSummary] = []
    @Published var selectedExchangeID: ExchangeSummary.ID? {
        didSet {
            if selectedExchangeID != oldValue {
                replayState = .idle
            }
        }
    }
    @Published private(set) var replayState: CaptureReplayState = .idle
    @Published private(set) var lastError: String?
    @Published private(set) var isBusy = false

    private let coreClient: CoreClient

    init(coreClient: CoreClient) {
        self.coreClient = coreClient
    }

    var selectedExchange: ExchangeSummary? {
        exchanges.first { $0.id == selectedExchangeID }
    }

    func clearLastError() {
        lastError = nil
    }

    func refreshSessionSummaries() async {
        do {
            let summaries = try await coreClient.searchSessionExchanges()
            mergeSessionSummaries(summaries)
        } catch {
            lastError = error.localizedDescription
        }
    }

    func clearSession() async {
        await perform {
            _ = try await coreClient.clearSession()
            exchanges.removeAll()
            selectedExchangeID = nil
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

        isBusy = true
        lastError = nil
        replayState = .running
        defer { isBusy = false }

        do {
            let result = try await coreClient.replaySession(
                exchangeID: selectedExchangeID,
                edit: .empty
            )
            if let index = exchanges.firstIndex(where: { $0.id == selectedExchangeID }) {
                exchanges[index].status = result.status
                exchanges[index].responsePreview = result.body
                exchanges[index].responseBytes = result.body.utf8.count
            }
            replayState = .succeeded(result.status)
        } catch {
            let message = Self.replayFailureMessage(error.localizedDescription)
            lastError = message
            replayState = .failed(message)
            if let index = exchanges.firstIndex(where: { $0.id == selectedExchangeID }) {
                exchanges[index].responsePreview = "Replay failed: \(message)"
            }
        }
    }

    func apply(_ event: ProxyEvent) {
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

    private static func replayFailureMessage(_ message: String) -> String {
        let prefix = "Replay request failed: "
        guard message.hasPrefix(prefix) else { return message }
        return String(message.dropFirst(prefix.count))
    }
}
