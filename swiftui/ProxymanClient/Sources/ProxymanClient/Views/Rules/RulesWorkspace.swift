import AppKit
import SwiftUI

struct RulesWorkspace: View {
    @EnvironmentObject private var model: AppModel

    var body: some View {
        HStack(spacing: 0) {
            RulePackList()
                .frame(width: 260)

            Rectangle()
                .fill(.separator.opacity(0.18))
                .frame(width: 1)

            RuleEditor()
                .frame(minWidth: 520)
        }
        .task {
            await model.refreshRulePacks()
        }
    }
}

private struct RulePackList: View {
    @EnvironmentObject private var model: AppModel

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Rule Packs")
                    .font(.system(size: 13, weight: .semibold))

                Spacer()

                Button {
                    Task { await model.refreshRulePacks() }
                } label: {
                    Label("Refresh", systemImage: "arrow.clockwise")
                }
                .labelStyle(.iconOnly)
                .buttonStyle(.proxymanIcon)
                .help("Refresh rule packs")
                .clickableCursor()
            }
            .padding(.horizontal, 12)
            .frame(height: 42)

            ScrollView {
                LazyVStack(spacing: 2) {
                    ForEach(model.rulePacks) { pack in
                        RulePackRow(
                            pack: pack,
                            isSelected: model.selectedRulePackName == pack.packName
                        ) {
                            Task { await model.selectRulePack(pack.packName) }
                        }
                    }
                }
                .padding(8)
                .frame(maxWidth: .infinity, alignment: .topLeading)
            }

            HStack(spacing: 6) {
                TextField("Pack name", text: $model.newRulePackName)
                    .textFieldStyle(.plain)
                    .font(.system(size: 12))
                    .onSubmit {
                        if model.canAddRulePack {
                            Task { await model.addRulePack() }
                        }
                    }

                Button {
                    Task { await model.addRulePack() }
                } label: {
                    Label("Add", systemImage: "plus")
                }
                .labelStyle(.iconOnly)
                .buttonStyle(.proxymanIcon)
                .disabled(!model.canAddRulePack)
                .help("Add rule pack")
                .clickableCursor(isEnabled: model.canAddRulePack)
            }
            .padding(.horizontal, 10)
            .frame(height: 40)
            .background(.quaternary.opacity(0.16))
        }
        .background(Color(nsColor: .controlBackgroundColor).opacity(0.55))
    }
}

private struct RulePackRow: View {
    var pack: RulePackSummary
    var isSelected: Bool
    var action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 8) {
                Image(systemName: pack.enabled ? "checkmark.circle.fill" : "circle")
                    .foregroundStyle(
                        pack.enabled ? Color.green : Color(nsColor: .tertiaryLabelColor)
                    )
                    .font(.system(size: 12))
                    .frame(width: 16)

                Text(pack.packName)
                    .font(.system(size: 12, weight: isSelected ? .semibold : .regular))
                    .lineLimit(1)

                Spacer()
            }
            .foregroundStyle(isSelected ? .primary : .secondary)
            .padding(.horizontal, 8)
            .padding(.vertical, 7)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(
                isSelected ? Color.primary.opacity(0.055) : Color.clear,
                in: RoundedRectangle(cornerRadius: 6)
            )
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .contentShape(Rectangle())
        .clickableCursor()
    }
}

private struct RuleEditor: View {
    @EnvironmentObject private var model: AppModel

    var body: some View {
        VStack(spacing: 0) {
            header

            if model.hasSelectedRulePack {
                VStack(spacing: 0) {
                    TextEditor(text: $model.ruleEditorContent)
                        .font(.system(size: 12, design: .monospaced))
                        .scrollContentBackground(.hidden)
                        .background(Color(nsColor: .textBackgroundColor))
                        .padding(.horizontal, 12)
                        .padding(.vertical, 10)

                    footer
                }
            } else {
                ContentUnavailableView("No Rule Pack Selected", systemImage: "slider.horizontal.3")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .background(Color(nsColor: .textBackgroundColor))
    }

    private var header: some View {
        HStack(spacing: 10) {
            Text(model.selectedRulePackName ?? "Rules")
                .font(.system(size: 13, weight: .semibold))
                .lineLimit(1)

            if model.isRuleEditorDirty {
                Text("Modified")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(.secondary)
            }

            Spacer()

            Toggle(
                "Enabled",
                isOn: Binding(
                    get: { model.ruleEditorEnabled },
                    set: { model.setRuleEditorEnabled($0) }
                )
            )
            .toggleStyle(.switch)
            .controlSize(.small)
            .disabled(!model.hasSelectedRulePack || model.isBusy)
            .clickableCursor(isEnabled: model.hasSelectedRulePack && !model.isBusy)

            Button {
                Task { await model.validateSelectedRulePack() }
            } label: {
                Label("Validate", systemImage: "checkmark.circle")
            }
            .buttonStyle(.proxymanAction)
            .disabled(!model.hasSelectedRulePack || model.isBusy)
            .clickableCursor(isEnabled: model.hasSelectedRulePack && !model.isBusy)

            Button {
                Task { await model.saveSelectedRulePack() }
            } label: {
                Label("Save", systemImage: "square.and.arrow.down")
            }
            .buttonStyle(.proxymanAction)
            .disabled(!model.canSaveRulePack)
            .clickableCursor(isEnabled: model.canSaveRulePack)

            Button {
                Task { await model.removeSelectedRulePack() }
            } label: {
                Label("Delete", systemImage: "trash")
            }
            .labelStyle(.iconOnly)
            .buttonStyle(.proxymanIcon)
            .foregroundStyle(.red)
            .disabled(!model.hasSelectedRulePack || model.isBusy)
            .help("Delete rule pack")
            .clickableCursor(isEnabled: model.hasSelectedRulePack && !model.isBusy)
        }
        .padding(.horizontal, 12)
        .frame(height: 44)
    }

    private var footer: some View {
        HStack(spacing: 10) {
            validationLabel

            Spacer()

            if let validation = model.ruleValidation {
                Text("\(validation.evaluationOrder.count) rules")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(.secondary)
            }
        }
        .padding(.horizontal, 12)
        .frame(height: 30)
        .background(.quaternary.opacity(0.14))
    }

    @ViewBuilder
    private var validationLabel: some View {
        if let validation = model.ruleValidation {
            Label(
                validation.valid ? "Valid" : "Invalid",
                systemImage: validation.valid ? "checkmark.circle.fill" : "xmark.octagon.fill"
            )
            .font(.system(size: 11, weight: .medium))
            .foregroundStyle(validation.valid ? .green : .red)
        } else {
            Text(model.isRuleEditorDirty ? "Unsaved" : "Saved")
                .font(.system(size: 11, weight: .medium))
                .foregroundStyle(.secondary)
        }
    }
}
