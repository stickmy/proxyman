import Combine
import Foundation

@MainActor
final class AppModel: ObservableObject {
    @Published var selectedSection: AppSection = .capture
    @Published private var appLastError: String?

    private let coreClient: CoreClient
    private let proxyLifecycleStore: ProxyLifecycleStore
    private let captureSessionsStore: CaptureSessionsStore
    private let rulePacksStore: RulePacksStore
    private let certificateStore: CertificateStore
    private let systemProxyStore: SystemProxyStore
    private var eventTask: Task<Void, Never>?
    private var childStoreCancellables: Set<AnyCancellable> = []

    init(coreClient: CoreClient) {
        self.coreClient = coreClient
        self.proxyLifecycleStore = ProxyLifecycleStore(coreClient: coreClient)
        self.captureSessionsStore = CaptureSessionsStore(coreClient: coreClient)
        self.rulePacksStore = RulePacksStore(coreClient: coreClient)
        self.certificateStore = CertificateStore(coreClient: coreClient)
        self.systemProxyStore = SystemProxyStore(coreClient: coreClient)
        relayChildChanges(proxyLifecycleStore.objectWillChange)
        relayChildChanges(captureSessionsStore.objectWillChange)
        relayChildChanges(rulePacksStore.objectWillChange)
        relayChildChanges(certificateStore.objectWillChange)
        relayChildChanges(systemProxyStore.objectWillChange)
    }

    var proxyStatus: ProxyStatus {
        proxyLifecycleStore.status
    }

    var listenPortText: String {
        get { proxyLifecycleStore.listenPortText }
        set { proxyLifecycleStore.listenPortText = newValue }
    }

    var exchanges: [ExchangeSummary] {
        captureSessionsStore.exchanges
    }

    var selectedExchangeID: ExchangeSummary.ID? {
        get { captureSessionsStore.selectedExchangeID }
        set { captureSessionsStore.selectedExchangeID = newValue }
    }

    var selectedExchange: ExchangeSummary? {
        captureSessionsStore.selectedExchange
    }

    var canEditEndpoint: Bool {
        proxyLifecycleStore.canEditEndpoint && !isBusy
    }

    var lastError: String? {
        appLastError ?? proxyLifecycleStore.lastError ?? captureSessionsStore.lastError ?? rulePacksStore.lastError ?? certificateStore.lastError ?? systemProxyStore.lastError
    }

    var isBusy: Bool {
        proxyLifecycleStore.isBusy || captureSessionsStore.isBusy || rulePacksStore.isBusy || certificateStore.isBusy || systemProxyStore.isBusy
    }

    var rulePacks: [RulePackSummary] {
        rulePacksStore.rulePacks
    }

    var selectedRulePackName: String? {
        get { rulePacksStore.selectedRulePackName }
        set { rulePacksStore.selectedRulePackName = newValue }
    }

    var ruleEditorContent: String {
        get { rulePacksStore.ruleEditorContent }
        set { rulePacksStore.ruleEditorContent = newValue }
    }

    var ruleEditorEnabled: Bool {
        get { rulePacksStore.ruleEditorEnabled }
        set { rulePacksStore.ruleEditorEnabled = newValue }
    }

    var newRulePackName: String {
        get { rulePacksStore.newRulePackName }
        set { rulePacksStore.newRulePackName = newValue }
    }

    var ruleValidation: RuleValidationResult? {
        rulePacksStore.ruleValidation
    }

    var ruleEditorState: RuleEditorState {
        rulePacksStore.ruleEditorState
    }

    var replayState: CaptureReplayState {
        captureSessionsStore.replayState
    }

    var caInstalled: Bool? {
        certificateStore.caInstalled
    }

    var ignoreUpstreamTlsVerification: Bool? {
        certificateStore.ignoreUpstreamTlsVerification
    }

    var systemProxyStatus: SystemProxyStatus? {
        systemProxyStore.status
    }

    var selectedRulePack: RulePackSummary? {
        rulePacksStore.selectedRulePack
    }

    var hasSelectedRulePack: Bool {
        rulePacksStore.hasSelectedRulePack
    }

    var isRuleEditorDirty: Bool {
        rulePacksStore.isRuleEditorDirty
    }

    var canAddRulePack: Bool {
        rulePacksStore.hasNewRulePackName && !isBusy
    }

    var canSaveRulePack: Bool {
        rulePacksStore.hasSelectedRulePack && rulePacksStore.isRuleEditorDirty && !isBusy
    }

    var systemProxyTargetText: String {
        proxyLifecycleStore.targetText
    }

    var canEnableSystemProxy: Bool {
        proxyLifecycleStore.hasValidListenPort && !isBusy
    }

    var canDisableSystemProxy: Bool {
        !isBusy
    }

    func refreshStatus() async {
        clearOperationErrors()
        await proxyLifecycleStore.refreshStatus()
    }

    func refreshSessionSummaries() async {
        appLastError = nil
        await captureSessionsStore.refreshSessionSummaries()
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
        clearOperationErrors()
        await proxyLifecycleStore.start()
    }

    func stopProxy() async {
        clearOperationErrors()
        await proxyLifecycleStore.stop()
    }

    func clearSession() async {
        appLastError = nil
        await captureSessionsStore.clearSession()
    }

    func selectSection(_ section: AppSection) {
        selectedSection = section
    }

    func refreshRulePacks() async {
        appLastError = nil
        await rulePacksStore.refreshRulePacks()
    }

    func selectRulePack(_ packName: String) async {
        appLastError = nil
        await rulePacksStore.selectRulePack(packName)
    }

    func validateSelectedRulePack() async {
        appLastError = nil
        await rulePacksStore.validateSelectedRulePack()
    }

    func saveSelectedRulePack() async {
        appLastError = nil
        await rulePacksStore.saveSelectedRulePack()
    }

    func addRulePack() async {
        appLastError = nil
        await rulePacksStore.addRulePack()
    }

    func removeSelectedRulePack() async {
        appLastError = nil
        await rulePacksStore.removeSelectedRulePack()
    }

    func setRuleEditorEnabled(_ enabled: Bool) {
        rulePacksStore.setRuleEditorEnabled(enabled)
    }

    func refreshCaStatus() async {
        appLastError = nil
        await certificateStore.refreshStatus()
    }

    func installCa() async {
        appLastError = nil
        await certificateStore.install()
    }

    func refreshUpstreamTlsStatus() async {
        appLastError = nil
        await certificateStore.refreshUpstreamTlsStatus()
    }

    func setIgnoreUpstreamTlsVerification(_ enabled: Bool) async {
        appLastError = nil
        await certificateStore.setIgnoreUpstreamTlsVerification(enabled)
    }

    func refreshSystemProxyStatus() async {
        appLastError = nil
        await systemProxyStore.refreshStatus(
            host: proxyStatus.host,
            port: systemProxyStatusPort()
        )
    }

    func enableSystemProxy() async {
        guard let port = proxyLifecycleStore.listenPort else {
            appLastError = "Port must be between 1 and 65535"
            return
        }

        appLastError = nil
        await systemProxyStore.enable(host: proxyStatus.host, port: port)
    }

    func disableSystemProxy() async {
        appLastError = nil
        await systemProxyStore.disable(
            host: proxyStatus.host,
            statusPort: systemProxyStatusPort()
        )
    }

    func loadRulePack(named packName: String) async {
        appLastError = nil
        await rulePacksStore.loadRulePack(named: packName)
    }

    func loadSelectedExchangeBodies() async {
        appLastError = nil
        await captureSessionsStore.loadSelectedExchangeBodies()
    }

    func replaySelectedExchange() async {
        appLastError = nil
        await captureSessionsStore.replaySelectedExchange()
    }

    private func apply(_ event: CoreEvent) {
        switch event.payload {
        case .proxyEvent(let envelope):
            captureSessionsStore.apply(envelope.event)
        case .proxyStatus(let payload):
            proxyLifecycleStore.apply(payload)
        case nil:
            break
        }
    }

    private func clearOperationErrors() {
        appLastError = nil
        proxyLifecycleStore.clearLastError()
        captureSessionsStore.clearLastError()
        rulePacksStore.clearLastError()
        certificateStore.clearLastError()
        systemProxyStore.clearLastError()
    }

    private func relayChildChanges(_ publisher: ObservableObjectPublisher) {
        publisher
            .sink { [weak self] _ in
                Task { @MainActor in
                    self?.objectWillChange.send()
                }
            }
            .store(in: &childStoreCancellables)
    }

    private func systemProxyStatusPort() -> Int {
        proxyLifecycleStore.statusPort
    }
}
