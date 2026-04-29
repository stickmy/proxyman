import Foundation

struct CoreEvent: Decodable {
    var apiVersion: Int
    var type: String
    var payload: CoreEventPayload?

    private enum CodingKeys: String, CodingKey {
        case apiVersion
        case type
        case payload
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        apiVersion = try container.decode(Int.self, forKey: .apiVersion)
        type = try container.decode(String.self, forKey: .type)

        switch type {
        case "proxyEvent":
            payload = .proxyEvent(try container.decode(ProxyEventEnvelope.self, forKey: .payload))
        case "proxy.started", "proxy.stopped":
            payload = .proxyStatus(try container.decode(ProxyStatusPayload.self, forKey: .payload))
        default:
            payload = nil
        }
    }
}

enum CoreEventPayload {
    case proxyEvent(ProxyEventEnvelope)
    case proxyStatus(ProxyStatusPayload)
}

struct ProxyEventEnvelope: Decodable {
    var apiVersion: Int
    var event: ProxyEvent
}

enum ProxyEvent: Decodable {
    case exchangeStarted(ExchangeStartedPayload)
    case requestHead(RequestHeadPayload)
    case requestBodyChunk(BodyChunkPayload)
    case requestFinished(RequestFinishedPayload)
    case responseHead(ResponseHeadPayload)
    case responseBodyChunk(BodyChunkPayload)
    case responseFinished(ResponseFinishedPayload)
    case exchangeFinished(ExchangeFinishedPayload)
    case exchangeError(ExchangeErrorPayload)
    case sseEvent(SsePayload)
    case webSocketMessage(WebSocketMessagePayload)
    case unsupported(String)

    private enum CodingKeys: String, CodingKey {
        case kind
        case payload
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let kind = try container.decode(String.self, forKey: .kind)

        switch kind {
        case "exchangeStarted":
            self = .exchangeStarted(try container.decode(ExchangeStartedPayload.self, forKey: .payload))
        case "requestHead":
            self = .requestHead(try container.decode(RequestHeadPayload.self, forKey: .payload))
        case "requestBodyChunk":
            self = .requestBodyChunk(try container.decode(BodyChunkPayload.self, forKey: .payload))
        case "requestFinished":
            self = .requestFinished(try container.decode(RequestFinishedPayload.self, forKey: .payload))
        case "responseHead":
            self = .responseHead(try container.decode(ResponseHeadPayload.self, forKey: .payload))
        case "responseBodyChunk":
            self = .responseBodyChunk(try container.decode(BodyChunkPayload.self, forKey: .payload))
        case "responseFinished":
            self = .responseFinished(try container.decode(ResponseFinishedPayload.self, forKey: .payload))
        case "exchangeFinished":
            self = .exchangeFinished(try container.decode(ExchangeFinishedPayload.self, forKey: .payload))
        case "exchangeError":
            self = .exchangeError(try container.decode(ExchangeErrorPayload.self, forKey: .payload))
        case "sseEvent":
            self = .sseEvent(try container.decode(SsePayload.self, forKey: .payload))
        case "webSocketMessage":
            self = .webSocketMessage(try container.decode(WebSocketMessagePayload.self, forKey: .payload))
        default:
            self = .unsupported(kind)
        }
    }
}

struct HeaderEntryPayload: Decodable, Equatable {
    var name: String
    var value: String
}

struct CapturedBodyPayload: Decodable, Equatable {
    var preview: String
    var size: Int
    var truncated: Bool
    var bodyRef: String?
}

struct ExchangeStartedPayload: Decodable, Equatable {
    var exchangeId: String
    var timestamp: Int64
    var method: String
    var uri: String
}

struct RequestHeadPayload: Decodable, Equatable {
    var exchangeId: String
    var timestamp: Int64
    var method: String
    var uri: String
    var version: String
    var headers: [HeaderEntryPayload]
    var capturedBody: CapturedBodyPayload?
}

struct RequestFinishedPayload: Decodable, Equatable {
    var exchangeId: String
    var timestamp: Int64
    var capturedBody: CapturedBodyPayload?
}

struct BodyChunkPayload: Decodable, Equatable {
    var exchangeId: String
    var timestamp: Int64
    var uri: String
    var offset: Int
    var byteLen: Int
    var preview: String
    var previewEncoding: String
    var lossyPreview: Bool
    var previewTruncated: Bool
}

struct ResponseHeadPayload: Decodable, Equatable {
    var exchangeId: String
    var timestamp: Int64
    var uri: String
    var status: Int
    var version: String
    var headers: [HeaderEntryPayload]
    var capturedBody: CapturedBodyPayload?
}

struct ResponseFinishedPayload: Decodable, Equatable {
    var exchangeId: String
    var timestamp: Int64
    var status: Int
    var capturedBody: CapturedBodyPayload?
}

struct ExchangeFinishedPayload: Decodable, Equatable {
    var exchangeId: String
    var timestamp: Int64
    var status: Int?
}

struct ExchangeErrorPayload: Decodable, Equatable {
    var exchangeId: String
    var timestamp: Int64
    var uri: String?
    var phase: String
    var message: String
    var recoverable: Bool
}

struct SsePayload: Decodable, Equatable {
    var exchangeId: String
    var timestamp: Int64
    var uri: String
    var event: String?
    var data: String
    var lastEventId: String?
    var retry: UInt64?
}

struct WebSocketMessagePayload: Decodable, Equatable {
    var exchangeId: String
    var timestamp: Int64
    var uri: String
    var direction: String
    var opcode: String
    var payloadPreview: String
    var payloadLen: Int
    var previewEncoding: String
    var lossyPreview: Bool
    var previewTruncated: Bool
    var closeCode: UInt16?
    var closeReason: String?
}
