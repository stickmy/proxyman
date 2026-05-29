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

            RulePackCreateBar()

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
        }
        .background(Color(nsColor: .controlBackgroundColor).opacity(0.55))
    }
}

private struct RulePackCreateBar: View {
    @EnvironmentObject private var model: AppModel

    var body: some View {
        HStack(spacing: 7) {
            TextField("New rule pack", text: $model.newRulePackName)
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
        .padding(.horizontal, 9)
        .frame(height: 30)
        .background(
            RoundedRectangle(cornerRadius: 7, style: .continuous)
                .fill(Color(nsColor: .textBackgroundColor).opacity(0.58))
        )
        .overlay {
            RoundedRectangle(cornerRadius: 7, style: .continuous)
                .stroke(Color(nsColor: .separatorColor).opacity(0.22), lineWidth: 1)
        }
        .padding(.horizontal, 9)
        .padding(.bottom, 6)
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
                .truncationMode(.tail)

            if model.hasSelectedRulePack {
                RuleEnabledPill(
                    isEnabled: model.ruleEditorEnabled,
                    isInteractive: !model.isBusy
                ) {
                    model.setRuleEditorEnabled(!model.ruleEditorEnabled)
                }
                .layoutPriority(1)

                RuleEditorStatusBadge(state: model.ruleEditorState)
                    .layoutPriority(1)

                Button {
                    Task { await model.removeSelectedRulePack() }
                } label: {
                    Label("Delete", systemImage: "trash")
                }
                .labelStyle(.iconOnly)
                .buttonStyle(.proxymanIcon)
                .foregroundStyle(.red)
                .disabled(model.isBusy)
                .help("Delete rule pack")
                .clickableCursor(isEnabled: !model.isBusy)
                .layoutPriority(1)
            }

            Spacer(minLength: 0)
        }
        .padding(.horizontal, 12)
        .frame(height: 44)
    }

    private var footer: some View {
        HStack(spacing: 10) {
            if let validation = model.ruleValidation, !validation.valid {
                Text("Fix validation errors before auto-save can finish")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(.red)
            }

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
}

private enum RuleHeaderPillMetrics {
    static let spacing: CGFloat = 5
    static let dotSize: CGFloat = 6
    static let horizontalPadding: CGFloat = 9
    static let height: CGFloat = 22
    static let font = Font.system(size: 11, weight: .semibold)
}

private struct RuleEditorStatusBadge: View {
    var state: RuleEditorState

    var body: some View {
        HStack(spacing: RuleHeaderPillMetrics.spacing) {
            Circle()
                .fill(color)
                .frame(width: RuleHeaderPillMetrics.dotSize, height: RuleHeaderPillMetrics.dotSize)

            Text(title)
                .font(RuleHeaderPillMetrics.font)
        }
        .foregroundStyle(color)
        .padding(.horizontal, RuleHeaderPillMetrics.horizontalPadding)
        .frame(height: RuleHeaderPillMetrics.height)
        .background(color.opacity(0.10), in: Capsule())
        .overlay {
            Capsule()
                .stroke(color.opacity(0.22), lineWidth: 1)
        }
    }

    private var title: String {
        switch state {
        case .idle:
            "Idle"
        case .saved:
            "Saved"
        case .pending:
            "Editing"
        case .validating:
            "Checking"
        case .saving:
            "Saving"
        case .invalid:
            "Invalid"
        case .failed:
            "Error"
        }
    }

    private var color: Color {
        switch state {
        case .saved:
            .green
        case .pending, .validating, .saving:
            .orange
        case .invalid, .failed:
            .red
        case .idle:
            .secondary
        }
    }
}

private struct RuleEnabledPill: View {
    var isEnabled: Bool
    var isInteractive: Bool
    var action: () -> Void
    @State private var isHovered = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: RuleHeaderPillMetrics.spacing) {
                Circle()
                    .fill(isEnabled ? Color.green : Color(nsColor: .tertiaryLabelColor))
                    .frame(width: RuleHeaderPillMetrics.dotSize, height: RuleHeaderPillMetrics.dotSize)

                Text(isEnabled ? "Enabled" : "Disabled")
                    .font(RuleHeaderPillMetrics.font)
            }
            .foregroundStyle(isEnabled ? .primary : .secondary)
            .padding(.horizontal, RuleHeaderPillMetrics.horizontalPadding)
            .frame(height: RuleHeaderPillMetrics.height)
            .background(background, in: Capsule())
            .overlay {
                Capsule()
                    .stroke(Color(nsColor: .separatorColor).opacity(isEnabled ? 0.32 : 0.22), lineWidth: 1)
            }
            .contentShape(Capsule())
        }
        .buttonStyle(.plain)
        .disabled(!isInteractive)
        .onHover { isHovered = $0 }
        .clickableCursor(isEnabled: isInteractive)
    }

    private var background: Color {
        if isHovered && isInteractive {
            return ProxymanTheme.Button.hoverBackground
        }
        if isEnabled {
            return Color.green.opacity(0.08)
        }
        return Color(nsColor: .controlBackgroundColor).opacity(0.7)
    }
}
