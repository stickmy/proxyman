import AppKit
import SwiftUI

struct ContentView: View {
    @EnvironmentObject private var model: AppModel
    private let titlebarHeight: CGFloat = 40

    var body: some View {
        ZStack(alignment: .topLeading) {
            HStack(spacing: 0) {
                Sidebar()
                    .frame(width: 188)

                Rectangle()
                    .fill(.separator.opacity(0.20))
                    .frame(width: 1)

                selectedWorkspace
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
            .padding(.top, titlebarHeight)

            ProxyTitlebarControls(model: model)
                .padding(.leading, 194)
                .padding(.top, 6)
        }
        .ignoresSafeArea(.container, edges: .top)
        .task {
            model.startEventStream()
            await model.refreshStatus()
            await model.refreshSessionSummaries()
        }
    }

    @ViewBuilder
    private var selectedWorkspace: some View {
        switch model.selectedSection {
        case .capture:
            CaptureWorkspace()
        case .rules:
            RulesWorkspace()
        case .systemProxy:
            SystemProxyWorkspace()
        case .certificates:
            CertificatesWorkspace()
        }
    }
}

private struct ProxyTitlebarControls: View {
    @ObservedObject var model: AppModel

    var body: some View {
        HStack(spacing: 12) {
            ProxyStatusItem(model: model)
            ProxyEndpointItem(model: model)
            ProxyRefreshButton(model: model)
            ProxyRunButton(model: model)
        }
        .fixedSize()
        .frame(height: 28)
    }
}

private struct ProxyStatusItem: View {
    @ObservedObject var model: AppModel

    var body: some View {
        HStack(spacing: 6) {
            Circle()
                .fill(statusColor)
                .frame(width: 8, height: 8)

            Text(model.proxyStatus.state.rawValue)
                .font(.system(size: 12, weight: .semibold))
                .lineLimit(1)
                .minimumScaleFactor(0.85)
                .frame(width: 66, alignment: .leading)
        }
        .foregroundStyle(.secondary)
        .frame(width: 80, alignment: .leading)
    }

    private var statusColor: Color {
        switch model.proxyStatus.state {
        case .running:
            .green
        case .starting, .stopping:
            .orange
        case .stopped:
            .secondary
        case .disconnected:
            .red
        }
    }
}

private struct ProxyEndpointItem: View {
    @ObservedObject var model: AppModel

    var body: some View {
        HStack(spacing: 3) {
            Text(model.proxyStatus.host)
                .foregroundStyle(.secondary)
                .frame(width: 72, alignment: .leading)

            Text(":")
                .foregroundStyle(.tertiary)

            TextField("Port", text: $model.listenPortText)
                .textFieldStyle(.plain)
                .multilineTextAlignment(.trailing)
                .frame(width: 54)
                .disabled(!model.canEditEndpoint)
        }
        .font(.system(size: 12, weight: .medium, design: .monospaced))
        .frame(width: 136, alignment: .leading)
    }
}

private struct ProxyRefreshButton: View {
    @ObservedObject var model: AppModel

    var body: some View {
        Button {
            Task { await model.refreshStatus() }
        } label: {
            Label("Refresh", systemImage: "arrow.clockwise")
        }
        .labelStyle(.iconOnly)
        .buttonStyle(.proxymanIcon)
        .font(.system(size: 15, weight: .medium))
        .frame(width: 24, height: 24)
        .help("Refresh sidecar status")
        .clickableCursor()
    }
}

private struct ProxyRunButton: View {
    @ObservedObject var model: AppModel

    var body: some View {
        Button {
            if model.proxyStatus.state == .running {
                Task { await model.stopProxy() }
            } else {
                Task { await model.startProxy() }
            }
        } label: {
            Label(
                model.proxyStatus.state == .running ? "Stop" : "Start",
                systemImage: model.proxyStatus.state == .running ? "stop.fill" : "play.fill"
            )
        }
        .labelStyle(.iconOnly)
        .buttonStyle(.proxymanIcon)
        .font(.system(size: 15, weight: .semibold))
        .frame(width: 24, height: 24)
        .disabled(model.isBusy)
        .keyboardShortcut("r", modifiers: [.command])
        .clickableCursor(isEnabled: !model.isBusy)
    }
}

private struct Sidebar: View {
    @EnvironmentObject private var model: AppModel
    @State private var visualSelection: AppSection = .capture

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            VStack(alignment: .leading, spacing: 3) {
                ForEach(AppSection.allCases) { section in
                    SidebarItem(
                        section: section,
                        isSelected: visualSelection == section,
                        previewAction: {
                            visualSelection = section
                        }
                    ) {
                        model.selectSection(section)
                    }
                }
            }
            .padding(.horizontal, 8)
            .padding(.top, 8)

            Spacer()
        }
        .onAppear {
            visualSelection = model.selectedSection
        }
        .onChange(of: model.selectedSection) { _, selectedSection in
            visualSelection = selectedSection
        }
    }
}

private struct SidebarItem: View {
    var section: AppSection
    var isSelected: Bool
    var previewAction: () -> Void
    var action: () -> Void
    @State private var isHovered = false
    @State private var isPressing = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: 9) {
                Image(systemName: section.systemImage)
                    .font(.system(size: 13, weight: .regular))
                    .frame(width: 17, height: 17)

                Text(section.rawValue)
                    .font(.system(size: 13, weight: isSelected ? .medium : .regular))
                    .lineLimit(1)

                Spacer(minLength: 0)
            }
            .foregroundStyle(isSelected ? .primary : .secondary)
            .padding(.horizontal, 8)
            .frame(height: 30)
            .background(
                RoundedRectangle(cornerRadius: 7, style: .continuous)
                    .fill(rowFill)
            )
            .contentShape(RoundedRectangle(cornerRadius: 7, style: .continuous))
        }
        .buttonStyle(.plain)
        .onHover { isHovered = $0 }
        .simultaneousGesture(
            DragGesture(minimumDistance: 0)
                .onChanged { _ in
                    guard !isPressing else { return }
                    isPressing = true
                    previewAction()
                }
                .onEnded { _ in
                    isPressing = false
                }
        )
        .animation(.easeOut(duration: 0.10), value: isHovered)
        .clickableCursor()
    }

    private var rowFill: Color {
        if isSelected {
            return Color.primary.opacity(0.065)
        }

        if isHovered {
            return Color.primary.opacity(0.035)
        }

        return .clear
    }
}
