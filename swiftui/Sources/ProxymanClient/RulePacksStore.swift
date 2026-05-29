import Combine
import Foundation

@MainActor
final class RulePacksStore: ObservableObject {
    @Published private(set) var rulePacks: [RulePackSummary] = []
    @Published var selectedRulePackName: String?
    @Published var ruleEditorContent = "" {
        didSet {
            guard !isLoadingRuleEditor else { return }
            ruleValidation = nil
            scheduleAutoValidateAndSave()
        }
    }
    @Published var ruleEditorEnabled = true {
        didSet {
            guard !isLoadingRuleEditor else { return }
            scheduleRulePackStatusUpdate()
        }
    }
    @Published var newRulePackName = ""
    @Published private(set) var ruleValidation: RuleValidationResult?
    @Published private(set) var ruleEditorState: RuleEditorState = .idle
    @Published private(set) var lastError: String?
    @Published private(set) var isBusy = false

    private let coreClient: CoreClient
    private var ruleEditorOriginalContent = ""
    private var ruleEditorOriginalEnabled = true
    private var isLoadingRuleEditor = false
    private var autoSaveTask: Task<Void, Never>?
    private var statusUpdateTask: Task<Void, Never>?

    init(coreClient: CoreClient) {
        self.coreClient = coreClient
    }

    deinit {
        autoSaveTask?.cancel()
        statusUpdateTask?.cancel()
    }

    var selectedRulePack: RulePackSummary? {
        rulePacks.first { $0.packName == selectedRulePackName }
    }

    var hasSelectedRulePack: Bool {
        selectedRulePackName != nil
    }

    var isRuleEditorDirty: Bool {
        ruleEditorContent != ruleEditorOriginalContent
    }

    var hasNewRulePackName: Bool {
        !trimmedNewRulePackName().isEmpty
    }

    func clearLastError() {
        lastError = nil
    }

    func refreshRulePacks() async {
        await perform {
            let packs = try await coreClient.listRulePacks()
            rulePacks = sortedRulePacks(packs)
            if selectedRulePackName == nil || !rulePacks.contains(where: { $0.packName == selectedRulePackName }) {
                selectedRulePackName = rulePacks.first?.packName
            }
        }

        if let nextRulePackName = selectedRulePackName {
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
            autoSaveTask?.cancel()
            ruleEditorState = .validating
            ruleValidation = try await coreClient.validateRulePackRules(
                packName: selectedRulePackName,
                enabled: ruleEditorEnabled,
                content: ruleEditorContent
            )
            ruleEditorState = ruleValidation?.valid == true ? (isRuleEditorDirty ? .pending : .saved) : .invalid
        }
    }

    func saveSelectedRulePack() async {
        guard let selectedRulePackName else { return }
        autoSaveTask?.cancel()

        await perform {
            try await validateAndSave(
                expectedPackName: selectedRulePackName,
                content: ruleEditorContent,
                enabled: ruleEditorEnabled
            )
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
        guard let packName = selectedRulePackName else { return }

        await perform {
            _ = try await coreClient.removeRulePack(packName: packName)
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
        guard ruleEditorEnabled != enabled else { return }
        ruleEditorEnabled = enabled
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
            ruleEditorState = .saved

            if let index = rulePacks.firstIndex(where: { $0.packName == rules.packName }) {
                rulePacks[index].enabled = rules.enabled
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

    private func scheduleAutoValidateAndSave() {
        autoSaveTask?.cancel()

        guard let selectedRulePackName else {
            ruleEditorState = .idle
            return
        }

        guard isRuleEditorDirty else {
            ruleEditorState = .saved
            return
        }

        ruleEditorState = .pending
        let content = ruleEditorContent
        let enabled = ruleEditorEnabled
        autoSaveTask = Task { @MainActor [weak self] in
            do {
                try await Task.sleep(for: .milliseconds(650))
            } catch {
                return
            }

            guard let self else { return }
            await self.autoValidateAndSave(
                expectedPackName: selectedRulePackName,
                content: content,
                enabled: enabled
            )
        }
    }

    private func scheduleRulePackStatusUpdate() {
        statusUpdateTask?.cancel()

        guard let selectedRulePackName else {
            return
        }

        let enabled = ruleEditorEnabled
        if let index = rulePacks.firstIndex(where: { $0.packName == selectedRulePackName }) {
            rulePacks[index].enabled = enabled
        }

        statusUpdateTask = Task { @MainActor [weak self] in
            do {
                try await Task.sleep(for: .milliseconds(150))
            } catch {
                return
            }

            guard let self else { return }
            await self.persistRulePackStatus(
                expectedPackName: selectedRulePackName,
                enabled: enabled
            )
        }
    }

    private func persistRulePackStatus(
        expectedPackName: String,
        enabled: Bool
    ) async {
        await perform {
            _ = try await coreClient.updateRulePackStatus(
                packName: expectedPackName,
                enabled: enabled
            )
            if selectedRulePackName == expectedPackName, ruleEditorEnabled == enabled {
                ruleEditorOriginalEnabled = enabled
            }
            if let index = rulePacks.firstIndex(where: { $0.packName == expectedPackName }) {
                rulePacks[index].enabled = enabled
            }
        }
    }

    private func autoValidateAndSave(
        expectedPackName: String,
        content: String,
        enabled: Bool
    ) async {
        do {
            try await validateAndSave(
                expectedPackName: expectedPackName,
                content: content,
                enabled: enabled
            )
        } catch {
            lastError = error.localizedDescription
            ruleEditorState = .failed(error.localizedDescription)
        }
    }

    private func validateAndSave(
        expectedPackName: String,
        content: String,
        enabled: Bool
    ) async throws {
        guard selectedRulePackName == expectedPackName else { return }
        guard ruleEditorContent == content, ruleEditorEnabled == enabled else { return }
        guard isRuleEditorDirty else {
            ruleEditorState = .saved
            return
        }

        lastError = nil
        ruleEditorState = .validating
        let validation = try await coreClient.validateRulePackRules(
            packName: expectedPackName,
            enabled: enabled,
            content: content
        )
        ruleValidation = validation
        guard validation.valid else {
            ruleEditorState = .invalid
            return
        }

        ruleEditorState = .saving
        _ = try await coreClient.saveRulePackRules(
            packName: expectedPackName,
            enabled: enabled,
            content: content
        )
        ruleEditorOriginalContent = content
        ruleEditorOriginalEnabled = enabled
        if let index = rulePacks.firstIndex(where: { $0.packName == expectedPackName }) {
            rulePacks[index].enabled = enabled
        }
        ruleEditorState = .saved
    }

    private func resetRuleEditor() {
        isLoadingRuleEditor = true
        defer { isLoadingRuleEditor = false }
        autoSaveTask?.cancel()
        statusUpdateTask?.cancel()

        selectedRulePackName = nil
        ruleEditorContent = ""
        ruleEditorEnabled = true
        ruleEditorOriginalContent = ""
        ruleEditorOriginalEnabled = true
        ruleValidation = nil
        ruleEditorState = .idle
    }

    private func trimmedNewRulePackName() -> String {
        newRulePackName.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private func sortedRulePacks(_ packs: [RulePackSummary]) -> [RulePackSummary] {
        packs.sorted { left, right in
            left.packName.localizedStandardCompare(right.packName) == .orderedAscending
        }
    }
}
