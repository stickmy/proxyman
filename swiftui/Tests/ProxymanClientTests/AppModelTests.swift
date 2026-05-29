import XCTest
@testable import ProxymanClient

@MainActor
final class AppModelTests: XCTestCase {
    func testRefreshStatusPreflightsAvailablePortWhenStopped() async {
        let client = TestCoreClient()
        client.statusResult = ProxyStatus(state: .stopped, host: "127.0.0.1", port: 9000)
        client.availablePortResult = 9001
        let model = AppModel(coreClient: client)

        await model.refreshStatus()

        XCTAssertEqual(model.proxyStatus, ProxyStatus(state: .stopped, host: "127.0.0.1", port: 9001))
        XCTAssertEqual(model.listenPortText, "9001")
        XCTAssertEqual(client.availablePortRequests, [
            AvailablePortRequest(host: "127.0.0.1", port: 9000)
        ])
    }

    func testRefreshStatusWhenRunningSkipsAvailablePortPreflight() async {
        let client = TestCoreClient()
        client.statusResult = ProxyStatus(state: .running, host: "127.0.0.1", port: 9100)
        let model = AppModel(coreClient: client)
        model.listenPortText = "9001"

        await model.refreshStatus()

        XCTAssertEqual(model.proxyStatus, ProxyStatus(state: .running, host: "127.0.0.1", port: 9100))
        XCTAssertEqual(model.listenPortText, "9100")
        XCTAssertTrue(client.availablePortRequests.isEmpty)
    }

    func testStartProxyUsesDisplayedPortWithoutFindingAvailableAgain() async {
        let client = TestCoreClient()
        client.startResult = ProxyStatus(state: .running, host: "127.0.0.1", port: 9001)
        let model = AppModel(coreClient: client)
        model.listenPortText = "9001"

        await model.startProxy()

        XCTAssertEqual(model.proxyStatus, ProxyStatus(state: .running, host: "127.0.0.1", port: 9001))
        XCTAssertEqual(model.listenPortText, "9001")
        XCTAssertEqual(client.startRequests, [
            StartProxyRequest(host: "127.0.0.1", port: 9001, findAvailable: false)
        ])
        XCTAssertTrue(client.availablePortRequests.isEmpty)
    }

    func testStartProxyRejectsInvalidDisplayedPort() async {
        let client = TestCoreClient()
        let model = AppModel(coreClient: client)
        model.listenPortText = "not-a-port"

        await model.startProxy()

        XCTAssertTrue(client.startRequests.isEmpty)
        XCTAssertEqual(model.proxyStatus, ProxyStatus(state: .disconnected, host: "127.0.0.1", port: 9000))
        XCTAssertEqual(model.lastError, "Port must be between 1 and 65535")
    }

    func testStartProxyRestoresPreviousStatusWhenStartFails() async {
        let client = TestCoreClient()
        client.startError = TestClientError(message: "start failed")
        let model = AppModel(coreClient: client)
        model.listenPortText = "9001"

        await model.startProxy()

        XCTAssertEqual(client.startRequests, [
            StartProxyRequest(host: "127.0.0.1", port: 9001, findAvailable: false)
        ])
        XCTAssertEqual(model.proxyStatus, ProxyStatus(state: .disconnected, host: "127.0.0.1", port: 9000))
        XCTAssertEqual(model.listenPortText, "9001")
        XCTAssertEqual(model.lastError, "start failed")
    }

    func testStopProxyPreservesLastKnownEndpoint() async {
        let client = TestCoreClient()
        client.statusResult = ProxyStatus(state: .running, host: "127.0.0.1", port: 9200)
        let model = AppModel(coreClient: client)

        await model.refreshStatus()
        await model.stopProxy()

        XCTAssertEqual(client.stopProxyCallCount, 1)
        XCTAssertEqual(model.proxyStatus, ProxyStatus(state: .stopped, host: "127.0.0.1", port: 9200))
        XCTAssertEqual(model.listenPortText, "9200")
    }

    func testStopProxyRestoresPreviousStatusWhenStopFails() async {
        let client = TestCoreClient()
        client.statusResult = ProxyStatus(state: .running, host: "127.0.0.1", port: 9200)
        client.stopError = TestClientError(message: "stop failed")
        let model = AppModel(coreClient: client)

        await model.refreshStatus()
        await model.stopProxy()

        XCTAssertEqual(client.stopProxyCallCount, 1)
        XCTAssertEqual(model.proxyStatus, ProxyStatus(state: .running, host: "127.0.0.1", port: 9200))
        XCTAssertEqual(model.listenPortText, "9200")
        XCTAssertEqual(model.lastError, "stop failed")
    }

    func testProxyStatusEventUpdatesLifecycleStateAndDisplayedPort() async throws {
        let client = TestCoreClient()
        let model = AppModel(coreClient: client)
        model.startEventStream()

        client.emit(try decodeCoreEvent("""
        {
          "apiVersion": 1,
          "type": "proxy.started",
          "payload": {
            "apiVersion": 1,
            "running": true,
            "state": "running",
            "host": "127.0.0.1",
            "port": 9400
          }
        }
        """))

        try await waitUntil { model.proxyStatus.port == 9400 }

        XCTAssertEqual(model.proxyStatus, ProxyStatus(state: .running, host: "127.0.0.1", port: 9400))
        XCTAssertEqual(model.listenPortText, "9400")
    }

    func testEventStreamUpdatesCaptureListFromProxyEvents() async throws {
        let client = TestCoreClient()
        let model = AppModel(coreClient: client)
        model.startEventStream()

        client.emit(try decodeCoreEvent("""
        {
          "apiVersion": 1,
          "type": "proxyEvent",
          "payload": {
            "apiVersion": 1,
            "event": {
              "kind": "exchangeStarted",
              "payload": {
                "exchangeId": "exchange-1",
                "timestamp": 1000,
                "method": "GET",
                "uri": "http://example.com/api?q=1"
              }
            }
          }
        }
        """))

        try await waitUntil { model.exchanges.count == 1 }

        XCTAssertEqual(model.selectedExchangeID, "exchange-1")
        XCTAssertEqual(model.exchanges.first?.method, "GET")
        XCTAssertEqual(model.exchanges.first?.host, "example.com")
        XCTAssertEqual(model.exchanges.first?.path, "/api?q=1")
        XCTAssertEqual(model.exchanges.first?.requestPreview, "GET /api?q=1")
    }

    func testSelectedExchangeBodyLoadingUpdatesPreviewsAndByteCounts() async {
        let client = TestCoreClient()
        client.sessionExchanges = [
            SessionExchangePayload(
                exchangeId: "exchange-1",
                method: "POST",
                uri: "http://example.com/upload",
                host: "example.com",
                status: 201,
                requestTime: 1_000,
                responseTime: 1_250,
                requestBodyRef: "request-body",
                responseBodyRef: "response-body"
            )
        ]
        client.bodyByRef = [
            "request-body": "request payload",
            "response-body": "response payload",
        ]
        let model = AppModel(coreClient: client)

        await model.refreshSessionSummaries()
        model.selectedExchangeID = "exchange-1"
        await model.loadSelectedExchangeBodies()

        XCTAssertEqual(model.exchanges.first?.requestPreview, "request payload")
        XCTAssertEqual(model.exchanges.first?.requestBytes, "request payload".utf8.count)
        XCTAssertEqual(model.exchanges.first?.responsePreview, "response payload")
        XCTAssertEqual(model.exchanges.first?.responseBytes, "response payload".utf8.count)
    }

    func testRefreshSessionSummariesMergesExistingExchangeAndKeepsLoadedPreviews() async throws {
        let client = TestCoreClient()
        let model = AppModel(coreClient: client)
        model.startEventStream()

        client.emit(try decodeCoreEvent("""
        {
          "apiVersion": 1,
          "type": "proxyEvent",
          "payload": {
            "apiVersion": 1,
            "event": {
              "kind": "exchangeStarted",
              "payload": {
                "exchangeId": "exchange-1",
                "timestamp": 1000,
                "method": "GET",
                "uri": "http://example.com/old"
              }
            }
          }
        }
        """))
        try await waitUntil { model.exchanges.count == 1 }

        client.sessionExchanges = [
            SessionExchangePayload(
                exchangeId: "exchange-1",
                method: "POST",
                uri: "http://api.example.com/new",
                host: "api.example.com",
                status: 204,
                requestTime: 1_000,
                responseTime: 1_040,
                requestBodyRef: "request-body",
                responseBodyRef: "response-body"
            )
        ]

        await model.refreshSessionSummaries()

        XCTAssertEqual(model.exchanges.count, 1)
        XCTAssertEqual(model.exchanges.first?.method, "POST")
        XCTAssertEqual(model.exchanges.first?.host, "api.example.com")
        XCTAssertEqual(model.exchanges.first?.path, "/new")
        XCTAssertEqual(model.exchanges.first?.status, 204)
        XCTAssertEqual(model.exchanges.first?.durationMillis, 40)
        XCTAssertEqual(model.exchanges.first?.requestPreview, "GET /old")
        XCTAssertEqual(model.exchanges.first?.requestBodyRef, "request-body")
        XCTAssertEqual(model.exchanges.first?.responseBodyRef, "response-body")
    }

    func testProxyEventHeadChunksAndFinishUpdateCaptureDetails() async throws {
        let client = TestCoreClient()
        let model = AppModel(coreClient: client)
        model.startEventStream()

        client.emit(try decodeCoreEvent("""
        {
          "apiVersion": 1,
          "type": "proxyEvent",
          "payload": {
            "apiVersion": 1,
            "event": {
              "kind": "requestHead",
              "payload": {
                "exchangeId": "exchange-1",
                "timestamp": 1000,
                "method": "POST",
                "uri": "http://example.com/api",
                "version": "HTTP/1.1",
                "headers": [
                  {"name": "content-type", "value": "text/plain"}
                ],
                "capturedBody": {
                  "preview": "initial body",
                  "size": 12,
                  "truncated": false,
                  "bodyRef": "request-body"
                }
              }
            }
          }
        }
        """))
        client.emit(try decodeCoreEvent("""
        {
          "apiVersion": 1,
          "type": "proxyEvent",
          "payload": {
            "apiVersion": 1,
            "event": {
              "kind": "responseBodyChunk",
              "payload": {
                "exchangeId": "exchange-1",
                "timestamp": 1020,
                "uri": "http://example.com/api",
                "offset": 0,
                "byteLen": 5,
                "preview": "hello",
                "previewEncoding": "utf8",
                "lossyPreview": false,
                "previewTruncated": false
              }
            }
          }
        }
        """))
        client.emit(try decodeCoreEvent("""
        {
          "apiVersion": 1,
          "type": "proxyEvent",
          "payload": {
            "apiVersion": 1,
            "event": {
              "kind": "exchangeFinished",
              "payload": {
                "exchangeId": "exchange-1",
                "timestamp": 1055,
                "status": 201
              }
            }
          }
        }
        """))

        try await waitUntil {
            model.exchanges.first?.status == 201 && model.exchanges.first?.responseBytes == 5
        }

        let exchange = try XCTUnwrap(model.exchanges.first)
        XCTAssertEqual(exchange.method, "POST")
        XCTAssertEqual(exchange.host, "example.com")
        XCTAssertEqual(exchange.path, "/api")
        XCTAssertTrue(exchange.requestPreview.contains("POST /api HTTP/1.1"))
        XCTAssertTrue(exchange.requestPreview.contains("content-type: text/plain"))
        XCTAssertTrue(exchange.requestPreview.contains("initial body"))
        XCTAssertEqual(exchange.requestBytes, 12)
        XCTAssertEqual(exchange.responsePreview, "hello")
        XCTAssertEqual(exchange.durationMillis, 55)
    }

    func testClearSessionClearsCaptureSelectionAndCallsClient() async {
        let client = TestCoreClient()
        client.sessionExchanges = [
            SessionExchangePayload(
                exchangeId: "exchange-1",
                method: "GET",
                uri: "http://example.com/",
                host: "example.com",
                status: 200,
                requestTime: nil,
                responseTime: nil,
                requestBodyRef: nil,
                responseBodyRef: nil
            )
        ]
        let model = AppModel(coreClient: client)

        await model.refreshSessionSummaries()
        model.selectedExchangeID = "exchange-1"
        await model.clearSession()

        XCTAssertEqual(client.clearSessionCallCount, 1)
        XCTAssertTrue(model.exchanges.isEmpty)
        XCTAssertNil(model.selectedExchangeID)
    }

    func testReplaySelectedExchangeUpdatesResponsePreviewAndStatus() async {
        let client = TestCoreClient()
        client.sessionExchanges = [
            SessionExchangePayload(
                exchangeId: "exchange-1",
                method: "GET",
                uri: "http://example.com/",
                host: "example.com",
                status: 200,
                requestTime: nil,
                responseTime: nil,
                requestBodyRef: nil,
                responseBodyRef: nil
            )
        ]
        client.replayResult = ReplayResult(status: 202, headers: ["x-test": "ok"], body: "replayed response")
        let model = AppModel(coreClient: client)

        await model.refreshSessionSummaries()
        model.selectedExchangeID = "exchange-1"
        await model.replaySelectedExchange()

        XCTAssertEqual(client.replayRequests, [
            ReplayRequest(exchangeID: "exchange-1", edit: .empty)
        ])
        XCTAssertEqual(model.exchanges.first?.status, 202)
        XCTAssertEqual(model.exchanges.first?.responsePreview, "replayed response")
        XCTAssertEqual(model.exchanges.first?.responseBytes, "replayed response".utf8.count)
    }

    func testReplaySelectedExchangeFailureIsVisibleInStateAndPreview() async {
        let client = TestCoreClient()
        client.sessionExchanges = [
            SessionExchangePayload(
                exchangeId: "exchange-1",
                method: "GET",
                uri: "http://example.com/",
                host: "example.com",
                status: 200,
                requestTime: nil,
                responseTime: nil,
                requestBodyRef: nil,
                responseBodyRef: nil
            )
        ]
        client.replayError = TestClientError(message: "origin offline")
        let model = AppModel(coreClient: client)

        await model.refreshSessionSummaries()
        model.selectedExchangeID = "exchange-1"
        await model.replaySelectedExchange()

        XCTAssertEqual(client.replayRequests, [
            ReplayRequest(exchangeID: "exchange-1", edit: .empty)
        ])
        XCTAssertEqual(model.replayState, .failed("origin offline"))
        XCTAssertEqual(model.lastError, "origin offline")
        XCTAssertEqual(model.exchanges.first?.responsePreview, "Replay failed: origin offline")
    }

    func testRefreshRulePacksSortsSelectsAndLoadsFirstPack() async {
        let client = TestCoreClient()
        client.rulePacksResult = [
            RulePackSummary(packName: "zeta", enabled: false),
            RulePackSummary(packName: "alpha", enabled: true),
        ]
        client.ruleRulesByPack = [
            "alpha": RulePackRules(
                packName: "alpha",
                enabled: true,
                content: "redirect GET https://example.com/* https://local.test\n",
                evaluationOrder: ["alpha-rule"]
            )
        ]
        let model = AppModel(coreClient: client)

        await model.refreshRulePacks()

        XCTAssertEqual(model.rulePacks.map(\.packName), ["alpha", "zeta"])
        XCTAssertEqual(model.selectedRulePackName, "alpha")
        XCTAssertEqual(model.ruleEditorContent, "redirect GET https://example.com/* https://local.test\n")
        XCTAssertTrue(model.ruleEditorEnabled)
        XCTAssertFalse(model.isRuleEditorDirty)
        XCTAssertEqual(client.getRulePackRequests, ["alpha"])
    }

    func testSelectRulePackLoadsContentAndUsesItAsDirtyBaseline() async {
        let client = TestCoreClient()
        client.rulePacksResult = [
            RulePackSummary(packName: "alpha", enabled: true),
            RulePackSummary(packName: "beta", enabled: false),
        ]
        client.ruleRulesByPack = [
            "alpha": RulePackRules(packName: "alpha", enabled: true, content: "alpha rules", evaluationOrder: []),
            "beta": RulePackRules(packName: "beta", enabled: false, content: "beta rules", evaluationOrder: []),
        ]
        let model = AppModel(coreClient: client)

        await model.refreshRulePacks()
        await model.selectRulePack("beta")

        XCTAssertEqual(model.selectedRulePackName, "beta")
        XCTAssertEqual(model.ruleEditorContent, "beta rules")
        XCTAssertFalse(model.ruleEditorEnabled)
        XCTAssertFalse(model.isRuleEditorDirty)
        XCTAssertEqual(client.getRulePackRequests, ["alpha", "beta"])
    }

    func testRuleEditorDirtyStateAndValidationStateTrackContentOnly() async throws {
        let client = TestCoreClient()
        client.rulePacksResult = [
            RulePackSummary(packName: "default", enabled: true)
        ]
        client.ruleRulesByPack = [
            "default": RulePackRules(packName: "default", enabled: true, content: "initial", evaluationOrder: [])
        ]
        client.validationResult = RuleValidationResult(valid: true, evaluationOrder: ["rule-1"])
        let model = AppModel(coreClient: client)

        await model.refreshRulePacks()
        await model.validateSelectedRulePack()

        XCTAssertEqual(model.ruleValidation, RuleValidationResult(valid: true, evaluationOrder: ["rule-1"]))

        model.ruleEditorContent = "changed"

        XCTAssertTrue(model.isRuleEditorDirty)
        XCTAssertNil(model.ruleValidation)

        await model.validateSelectedRulePack()
        model.setRuleEditorEnabled(false)

        XCTAssertTrue(model.isRuleEditorDirty)
        XCTAssertEqual(model.ruleValidation, RuleValidationResult(valid: true, evaluationOrder: ["rule-1"]))
        XCTAssertEqual(model.ruleEditorState, .pending)
    }

    func testRuleEditorAutoValidatesAndSavesChangedContent() async throws {
        let client = TestCoreClient()
        client.rulePacksResult = [
            RulePackSummary(packName: "default", enabled: true)
        ]
        client.ruleRulesByPack = [
            "default": RulePackRules(packName: "default", enabled: true, content: "initial", evaluationOrder: [])
        ]
        client.validationResult = RuleValidationResult(valid: true, evaluationOrder: ["rule-1"])
        let model = AppModel(coreClient: client)

        await model.refreshRulePacks()
        model.ruleEditorContent = "updated"

        try await waitUntil(timeout: 2) {
            client.saveRequests.count == 1
        }

        XCTAssertEqual(client.validationRequests, [
            RuleValidationRequest(packName: "default", enabled: true, content: "updated")
        ])
        XCTAssertEqual(client.saveRequests, [
            RuleSaveRequest(packName: "default", enabled: true, content: "updated")
        ])
        XCTAssertEqual(model.ruleEditorState, .saved)
        XCTAssertFalse(model.isRuleEditorDirty)
    }

    func testRuleEditorAutoValidationDoesNotSaveInvalidContent() async throws {
        let client = TestCoreClient()
        client.rulePacksResult = [
            RulePackSummary(packName: "default", enabled: true)
        ]
        client.ruleRulesByPack = [
            "default": RulePackRules(packName: "default", enabled: true, content: "initial", evaluationOrder: [])
        ]
        client.validationResult = RuleValidationResult(valid: false, evaluationOrder: ["invalid"])
        let model = AppModel(coreClient: client)

        await model.refreshRulePacks()
        model.ruleEditorContent = "broken"

        try await waitUntil(timeout: 2) {
            model.ruleEditorState == .invalid
        }

        XCTAssertEqual(client.validationRequests, [
            RuleValidationRequest(packName: "default", enabled: true, content: "broken")
        ])
        XCTAssertTrue(client.saveRequests.isEmpty)
        XCTAssertTrue(model.isRuleEditorDirty)
    }

    func testRuleEditorEnabledTogglePersistsStatusWithoutChangingEditorState() async throws {
        let client = TestCoreClient()
        client.rulePacksResult = [
            RulePackSummary(packName: "default", enabled: true)
        ]
        client.ruleRulesByPack = [
            "default": RulePackRules(packName: "default", enabled: true, content: "initial", evaluationOrder: [])
        ]
        client.validationResult = RuleValidationResult(valid: true, evaluationOrder: ["rule-1"])
        let model = AppModel(coreClient: client)

        await model.refreshRulePacks()
        await model.validateSelectedRulePack()

        XCTAssertEqual(model.ruleEditorState, .saved)
        XCTAssertEqual(model.ruleValidation, RuleValidationResult(valid: true, evaluationOrder: ["rule-1"]))

        model.setRuleEditorEnabled(false)

        try await waitUntil(timeout: 2) {
            client.statusUpdateRequests.count == 1
        }

        XCTAssertEqual(client.statusUpdateRequests, [
            RuleAddRequest(packName: "default", enabled: false)
        ])
        XCTAssertEqual(client.validationRequests, [
            RuleValidationRequest(packName: "default", enabled: true, content: "initial")
        ])
        XCTAssertTrue(client.saveRequests.isEmpty)
        XCTAssertEqual(model.selectedRulePack?.enabled, false)
        XCTAssertEqual(model.ruleEditorState, .saved)
        XCTAssertEqual(model.ruleValidation, RuleValidationResult(valid: true, evaluationOrder: ["rule-1"]))
        XCTAssertFalse(model.isRuleEditorDirty)
    }

    func testValidateSelectedRulePackSendsCurrentContentAndEnabledState() async {
        let client = TestCoreClient()
        client.rulePacksResult = [
            RulePackSummary(packName: "default", enabled: true)
        ]
        client.ruleRulesByPack = [
            "default": RulePackRules(packName: "default", enabled: true, content: "initial", evaluationOrder: [])
        ]
        client.validationResult = RuleValidationResult(valid: false, evaluationOrder: ["invalid-rule"])
        let model = AppModel(coreClient: client)

        await model.refreshRulePacks()
        model.ruleEditorContent = "broken rule"
        model.setRuleEditorEnabled(false)
        await model.validateSelectedRulePack()

        XCTAssertEqual(model.ruleValidation, RuleValidationResult(valid: false, evaluationOrder: ["invalid-rule"]))
        XCTAssertEqual(client.validationRequests, [
            RuleValidationRequest(packName: "default", enabled: false, content: "broken rule")
        ])
    }

    func testSaveSelectedRulePackValidatesBeforeSavingAndResetsDirtyState() async {
        let client = TestCoreClient()
        client.rulePacksResult = [
            RulePackSummary(packName: "default", enabled: true)
        ]
        client.ruleRulesByPack = [
            "default": RulePackRules(packName: "default", enabled: true, content: "initial", evaluationOrder: [])
        ]
        client.validationResult = RuleValidationResult(valid: true, evaluationOrder: ["rule-1"])
        let model = AppModel(coreClient: client)

        await model.refreshRulePacks()
        model.ruleEditorContent = "updated"
        model.setRuleEditorEnabled(false)
        await model.saveSelectedRulePack()

        XCTAssertEqual(client.validationRequests, [
            RuleValidationRequest(packName: "default", enabled: false, content: "updated")
        ])
        XCTAssertEqual(client.saveRequests, [
            RuleSaveRequest(packName: "default", enabled: false, content: "updated")
        ])
        XCTAssertEqual(model.ruleValidation, RuleValidationResult(valid: true, evaluationOrder: ["rule-1"]))
        XCTAssertFalse(model.isRuleEditorDirty)
        XCTAssertEqual(model.selectedRulePack?.enabled, false)
    }

    func testSaveSelectedRulePackSkipsSaveWhenValidationFails() async {
        let client = TestCoreClient()
        client.rulePacksResult = [
            RulePackSummary(packName: "default", enabled: true)
        ]
        client.ruleRulesByPack = [
            "default": RulePackRules(packName: "default", enabled: true, content: "initial", evaluationOrder: [])
        ]
        client.validationResult = RuleValidationResult(valid: false, evaluationOrder: ["invalid"])
        let model = AppModel(coreClient: client)

        await model.refreshRulePacks()
        model.ruleEditorContent = "broken"
        await model.saveSelectedRulePack()

        XCTAssertEqual(client.validationRequests, [
            RuleValidationRequest(packName: "default", enabled: true, content: "broken")
        ])
        XCTAssertTrue(client.saveRequests.isEmpty)
        XCTAssertTrue(model.isRuleEditorDirty)
    }

    func testAddRulePackTrimsNameRefreshesListAndLoadsNewPack() async {
        let client = TestCoreClient()
        client.rulePacksResult = [
            RulePackSummary(packName: "default", enabled: true)
        ]
        client.ruleRulesByPack = [
            "default": RulePackRules(packName: "default", enabled: true, content: "initial", evaluationOrder: []),
            "mobile": RulePackRules(packName: "mobile", enabled: true, content: "mobile rules", evaluationOrder: []),
        ]
        let model = AppModel(coreClient: client)

        await model.refreshRulePacks()
        model.newRulePackName = "  mobile  "
        await model.addRulePack()

        XCTAssertEqual(client.addRequests, [
            RuleAddRequest(packName: "mobile", enabled: true)
        ])
        XCTAssertEqual(model.newRulePackName, "")
        XCTAssertEqual(model.rulePacks.map(\.packName), ["default", "mobile"])
        XCTAssertEqual(model.selectedRulePackName, "mobile")
        XCTAssertEqual(model.ruleEditorContent, "mobile rules")
        XCTAssertFalse(model.isRuleEditorDirty)
    }

    func testRemoveSelectedRulePackSelectsNextPackOrResetsEditor() async {
        let client = TestCoreClient()
        client.rulePacksResult = [
            RulePackSummary(packName: "alpha", enabled: true),
            RulePackSummary(packName: "beta", enabled: false),
        ]
        client.ruleRulesByPack = [
            "alpha": RulePackRules(packName: "alpha", enabled: true, content: "alpha rules", evaluationOrder: []),
            "beta": RulePackRules(packName: "beta", enabled: false, content: "beta rules", evaluationOrder: []),
        ]
        let model = AppModel(coreClient: client)

        await model.refreshRulePacks()
        await model.removeSelectedRulePack()

        XCTAssertEqual(client.removeRequests, ["alpha"])
        XCTAssertEqual(model.rulePacks.map(\.packName), ["beta"])
        XCTAssertEqual(model.selectedRulePackName, "beta")
        XCTAssertEqual(model.ruleEditorContent, "beta rules")
        XCTAssertFalse(model.ruleEditorEnabled)
        XCTAssertFalse(model.isRuleEditorDirty)

        await model.removeSelectedRulePack()

        XCTAssertEqual(client.removeRequests, ["alpha", "beta"])
        XCTAssertTrue(model.rulePacks.isEmpty)
        XCTAssertNil(model.selectedRulePackName)
        XCTAssertEqual(model.ruleEditorContent, "")
        XCTAssertTrue(model.ruleEditorEnabled)
        XCTAssertFalse(model.isRuleEditorDirty)
    }

    func testCaStatusAndInstallUpdateTrustState() async {
        let client = TestCoreClient()
        client.caStatusResult = false
        client.installCaResult = true
        let model = AppModel(coreClient: client)

        await model.refreshCaStatus()

        XCTAssertEqual(model.caInstalled, false)
        XCTAssertEqual(client.caStatusCallCount, 1)

        await model.installCa()

        XCTAssertEqual(model.caInstalled, true)
        XCTAssertEqual(client.installCaCallCount, 1)
    }

    func testUpstreamTlsStatusAndToggleUpdateCertificateState() async {
        let client = TestCoreClient()
        client.upstreamTlsIgnoreResult = false
        let model = AppModel(coreClient: client)

        await model.refreshUpstreamTlsStatus()

        XCTAssertEqual(model.ignoreUpstreamTlsVerification, false)
        XCTAssertEqual(client.upstreamTlsStatusCallCount, 1)

        await model.setIgnoreUpstreamTlsVerification(true)

        XCTAssertEqual(model.ignoreUpstreamTlsVerification, true)
        XCTAssertEqual(client.upstreamTlsUpdateRequests, [true])
    }

    func testRefreshSystemProxyStatusUsesDisplayedPort() async {
        let client = TestCoreClient()
        client.systemProxyStatusResult = SystemProxyStatus(
            enabled: true,
            matchesRequested: true,
            services: [
                SystemProxyServiceStatus(
                    service: "Wi-Fi",
                    web: SystemProxyState(enabled: true, server: "127.0.0.1", port: "9001"),
                    secureWeb: SystemProxyState(enabled: true, server: "127.0.0.1", port: "9001"),
                    bypassDomains: []
                )
            ]
        )
        let model = AppModel(coreClient: client)
        model.listenPortText = "9001"

        await model.refreshSystemProxyStatus()

        XCTAssertEqual(client.systemProxyStatusRequests, [
            SystemProxyStatusRequest(host: "127.0.0.1", port: 9001)
        ])
        XCTAssertEqual(model.systemProxyStatus, client.systemProxyStatusResult)
        XCTAssertEqual(model.systemProxyTargetText, "127.0.0.1:9001")
    }

    func testEnableSystemProxyUsesDisplayedPortThenRefreshesStatus() async {
        let client = TestCoreClient()
        client.systemProxyStatusResult = SystemProxyStatus(enabled: true, matchesRequested: true, services: [])
        let model = AppModel(coreClient: client)
        model.listenPortText = "9100"

        await model.enableSystemProxy()

        XCTAssertEqual(client.enableSystemProxyRequests, [9100])
        XCTAssertEqual(client.systemProxyStatusRequests, [
            SystemProxyStatusRequest(host: "127.0.0.1", port: 9100)
        ])
        XCTAssertEqual(model.systemProxyStatus, client.systemProxyStatusResult)
    }

    func testEnableSystemProxyRejectsInvalidDisplayedPort() async {
        let client = TestCoreClient()
        let model = AppModel(coreClient: client)
        model.listenPortText = "0"

        await model.enableSystemProxy()

        XCTAssertTrue(client.enableSystemProxyRequests.isEmpty)
        XCTAssertTrue(client.systemProxyStatusRequests.isEmpty)
        XCTAssertEqual(model.lastError, "Port must be between 1 and 65535")
    }

    func testDisableSystemProxyRefreshesStatusUsingProxyStatusPortWhenDisplayedPortIsInvalid() async {
        let client = TestCoreClient()
        client.statusResult = ProxyStatus(state: .running, host: "127.0.0.1", port: 9200)
        client.systemProxyStatusResult = SystemProxyStatus(enabled: false, matchesRequested: false, services: [])
        let model = AppModel(coreClient: client)

        await model.refreshStatus()
        model.listenPortText = "invalid"
        await model.disableSystemProxy()

        XCTAssertEqual(client.disableSystemProxyCallCount, 1)
        XCTAssertEqual(client.systemProxyStatusRequests, [
            SystemProxyStatusRequest(host: "127.0.0.1", port: 9200)
        ])
        XCTAssertEqual(model.systemProxyStatus, client.systemProxyStatusResult)
        XCTAssertEqual(model.systemProxyTargetText, "127.0.0.1:9200")
    }
}

final class SidecarResponseDecoderTests: XCTestCase {
    func testDecodeJsonRpcResultReturnsTypedPayload() throws {
        let data = Data("""
        {"jsonrpc":"2.0","id":"1","result":{"apiVersion":1,"host":"127.0.0.1","port":9001}}
        """.utf8)

        let result = try SidecarResponseDecoder.decode(AvailablePortPayload.self, from: data)

        XCTAssertEqual(result.port, 9001)
    }

    func testDecodeJsonRpcErrorThrowsRpcMessage() throws {
        let data = Data("""
        {"jsonrpc":"2.0","id":"1","error":{"code":-32602,"message":"bad port"}}
        """.utf8)

        XCTAssertThrowsError(try SidecarResponseDecoder.decode(AvailablePortPayload.self, from: data)) { error in
            guard case UnixSocketError.rpcError("bad port") = error else {
                return XCTFail("Unexpected error: \(error)")
            }
        }
    }
}

private struct AvailablePortRequest: Equatable {
    var host: String
    var port: Int
}

private struct StartProxyRequest: Equatable {
    var host: String
    var port: Int
    var findAvailable: Bool
}

private struct RuleValidationRequest: Equatable {
    var packName: String
    var enabled: Bool
    var content: String
}

private struct RuleSaveRequest: Equatable {
    var packName: String
    var enabled: Bool
    var content: String
}

private struct RuleAddRequest: Equatable {
    var packName: String
    var enabled: Bool
}

private struct SystemProxyStatusRequest: Equatable {
    var host: String
    var port: Int
}

private struct ReplayRequest: Equatable {
    var exchangeID: String
    var edit: ReplayEdit
}

private struct TestClientError: Error, LocalizedError, Equatable {
    var message: String

    var errorDescription: String? {
        message
    }
}

private final class TestCoreClient: CoreClient, @unchecked Sendable {
    var statusResult = ProxyStatus(state: .stopped, host: "127.0.0.1", port: 9000)
    var availablePortResult = 9000
    var startResult = ProxyStatus(state: .running, host: "127.0.0.1", port: 9000)
    var sessionExchanges: [SessionExchangePayload] = []
    var bodyByRef: [String: String] = [:]
    var rulePacksResult: [RulePackSummary] = []
    var ruleRulesByPack: [String: RulePackRules] = [:]
    var validationResult = RuleValidationResult(valid: true, evaluationOrder: [])
    var caStatusResult = false
    var installCaResult = true
    var upstreamTlsIgnoreResult = false
    var systemProxyStatusResult = SystemProxyStatus(enabled: false, matchesRequested: false, services: [])
    var replayResult = ReplayResult(status: 200, headers: [:], body: "")
    var replayError: TestClientError?
    var startError: TestClientError?
    var stopError: TestClientError?

    private(set) var availablePortRequests: [AvailablePortRequest] = []
    private(set) var startRequests: [StartProxyRequest] = []
    private(set) var clearSessionCallCount = 0
    private(set) var replayRequests: [ReplayRequest] = []
    private(set) var getRulePackRequests: [String] = []
    private(set) var validationRequests: [RuleValidationRequest] = []
    private(set) var saveRequests: [RuleSaveRequest] = []
    private(set) var addRequests: [RuleAddRequest] = []
    private(set) var removeRequests: [String] = []
    private(set) var statusUpdateRequests: [RuleAddRequest] = []
    private(set) var caStatusCallCount = 0
    private(set) var installCaCallCount = 0
    private(set) var upstreamTlsStatusCallCount = 0
    private(set) var upstreamTlsUpdateRequests: [Bool] = []
    private(set) var systemProxyStatusRequests: [SystemProxyStatusRequest] = []
    private(set) var enableSystemProxyRequests: [Int] = []
    private(set) var disableSystemProxyCallCount = 0
    private(set) var stopProxyCallCount = 0
    private var eventContinuation: AsyncStream<CoreEvent>.Continuation?
    private var bufferedEvents: [CoreEvent] = []

    func status() async throws -> ProxyStatus {
        statusResult
    }

    func availablePort(host: String, port: Int) async throws -> Int {
        availablePortRequests.append(AvailablePortRequest(host: host, port: port))
        return availablePortResult
    }

    func startProxy(host: String, port: Int, findAvailable: Bool) async throws -> ProxyStatus {
        startRequests.append(StartProxyRequest(host: host, port: port, findAvailable: findAvailable))
        if let startError {
            throw startError
        }
        return startResult
    }

    func stopProxy() async throws -> ProxyStatus {
        stopProxyCallCount += 1
        if let stopError {
            throw stopError
        }
        return ProxyStatus(state: .stopped, host: "127.0.0.1", port: 9000)
    }

    func searchSessionExchanges() async throws -> [SessionExchangePayload] {
        sessionExchanges
    }

    func loadSessionBody(bodyRef: String) async throws -> String {
        bodyByRef[bodyRef] ?? ""
    }

    func clearSession() async throws -> Int {
        clearSessionCallCount += 1
        sessionExchanges.removeAll()
        return 0
    }

    func exportSessionHar() async throws -> Data {
        Data()
    }

    func importSessionHar(_ harData: Data) async throws -> Int {
        0
    }

    func replaySession(exchangeID: String, edit: ReplayEdit) async throws -> ReplayResult {
        replayRequests.append(ReplayRequest(exchangeID: exchangeID, edit: edit))
        if let replayError {
            throw replayError
        }
        return replayResult
    }

    func caStatus() async throws -> Bool {
        caStatusCallCount += 1
        return caStatusResult
    }

    func installCa() async throws -> Bool {
        installCaCallCount += 1
        return installCaResult
    }

    func upstreamTlsStatus() async throws -> Bool {
        upstreamTlsStatusCallCount += 1
        return upstreamTlsIgnoreResult
    }

    func updateUpstreamTls(ignoreVerification: Bool) async throws -> Bool {
        upstreamTlsUpdateRequests.append(ignoreVerification)
        upstreamTlsIgnoreResult = ignoreVerification
        return ignoreVerification
    }

    func systemProxyStatus(host: String, port: Int) async throws -> SystemProxyStatus {
        systemProxyStatusRequests.append(SystemProxyStatusRequest(host: host, port: port))
        return systemProxyStatusResult
    }

    func enableSystemProxy(port: Int) async throws -> Bool {
        enableSystemProxyRequests.append(port)
        return true
    }

    func disableSystemProxy() async throws -> Bool {
        disableSystemProxyCallCount += 1
        return false
    }

    func listRulePacks() async throws -> [RulePackSummary] {
        rulePacksResult
    }

    func getRulePackRules(packName: String) async throws -> RulePackRules {
        getRulePackRequests.append(packName)
        if let rules = ruleRulesByPack[packName] {
            return rules
        }
        return RulePackRules(
            packName: packName,
            enabled: rulePacksResult.first(where: { $0.packName == packName })?.enabled ?? true,
            content: "",
            evaluationOrder: []
        )
    }

    func validateRulePackRules(
        packName: String,
        enabled: Bool,
        content: String
    ) async throws -> RuleValidationResult {
        validationRequests.append(RuleValidationRequest(packName: packName, enabled: enabled, content: content))
        return validationResult
    }

    func saveRulePackRules(packName: String, enabled: Bool, content: String) async throws -> Bool {
        saveRequests.append(RuleSaveRequest(packName: packName, enabled: enabled, content: content))
        ruleRulesByPack[packName] = RulePackRules(
            packName: packName,
            enabled: enabled,
            content: content,
            evaluationOrder: validationResult.evaluationOrder
        )
        updateRulePackSummary(packName: packName, enabled: enabled)
        return true
    }

    func addRulePack(packName: String, enabled: Bool) async throws -> Bool {
        addRequests.append(RuleAddRequest(packName: packName, enabled: enabled))
        if !rulePacksResult.contains(where: { $0.packName == packName }) {
            rulePacksResult.append(RulePackSummary(packName: packName, enabled: enabled))
        }
        if ruleRulesByPack[packName] == nil {
            ruleRulesByPack[packName] = RulePackRules(packName: packName, enabled: enabled, content: "", evaluationOrder: [])
        }
        return true
    }

    func removeRulePack(packName: String) async throws -> Bool {
        removeRequests.append(packName)
        rulePacksResult.removeAll { $0.packName == packName }
        ruleRulesByPack[packName] = nil
        return true
    }

    func updateRulePackStatus(packName: String, enabled: Bool) async throws -> Bool {
        statusUpdateRequests.append(RuleAddRequest(packName: packName, enabled: enabled))
        updateRulePackSummary(packName: packName, enabled: enabled)
        return true
    }

    func eventStream() -> AsyncStream<CoreEvent> {
        AsyncStream { continuation in
            self.eventContinuation = continuation
            for event in self.bufferedEvents {
                continuation.yield(event)
            }
            self.bufferedEvents.removeAll()
        }
    }

    func emit(_ event: CoreEvent) {
        if let eventContinuation {
            eventContinuation.yield(event)
        } else {
            bufferedEvents.append(event)
        }
    }

    private func updateRulePackSummary(packName: String, enabled: Bool) {
        if let index = rulePacksResult.firstIndex(where: { $0.packName == packName }) {
            rulePacksResult[index].enabled = enabled
        } else {
            rulePacksResult.append(RulePackSummary(packName: packName, enabled: enabled))
        }
    }
}

private func decodeCoreEvent(_ json: String) throws -> CoreEvent {
    try JSONDecoder().decode(CoreEvent.self, from: Data(json.utf8))
}

private func waitUntil(
    timeout: TimeInterval = 1,
    condition: @MainActor @escaping () -> Bool
) async throws {
    let deadline = Date().addingTimeInterval(timeout)
    while Date() < deadline {
        if await condition() {
            return
        }
        try await Task.sleep(for: .milliseconds(10))
    }
    XCTFail("Timed out waiting for condition")
}
