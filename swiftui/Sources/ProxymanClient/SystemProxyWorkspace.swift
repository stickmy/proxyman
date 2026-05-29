import AppKit
import SwiftUI

struct SystemProxyWorkspace: View {
    @EnvironmentObject private var model: AppModel

    var body: some View {
        VStack(spacing: 0) {
            WorkspaceHeader(
                title: "System Proxy",
                systemImage: "network",
                status: systemProxyStatus
            )

            VStack(alignment: .leading, spacing: 18) {
                SettingsRow(title: "Target", value: model.systemProxyTargetText)
                SettingsRow(title: "HTTP/HTTPS", value: systemProxyStatus.title)
                SettingsRow(title: "Services", value: serviceText)
                SettingsRow(title: "HTTP", value: primaryService?.web.targetText ?? "-")
                SettingsRow(title: "HTTPS", value: primaryService?.secureWeb.targetText ?? "-")

                HStack(spacing: 8) {
                    Button {
                        Task { await model.refreshSystemProxyStatus() }
                    } label: {
                        Label("Refresh", systemImage: "arrow.clockwise")
                    }
                    .buttonStyle(.proxymanAction)
                    .disabled(model.isBusy)
                    .clickableCursor(isEnabled: !model.isBusy)

                    Button {
                        Task { await model.enableSystemProxy() }
                    } label: {
                        Label("Enable", systemImage: "power")
                    }
                    .buttonStyle(.proxymanAction)
                    .disabled(!model.canEnableSystemProxy)
                    .clickableCursor(isEnabled: model.canEnableSystemProxy)

                    Button {
                        Task { await model.disableSystemProxy() }
                    } label: {
                        Label("Disable", systemImage: "xmark.circle")
                    }
                    .buttonStyle(.proxymanAction)
                    .disabled(!model.canDisableSystemProxy)
                    .clickableCursor(isEnabled: model.canDisableSystemProxy)
                }
            }
            .padding(20)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
        .background(Color(nsColor: .textBackgroundColor))
        .task {
            await model.refreshSystemProxyStatus()
        }
    }

    private var systemProxyStatus: WorkspaceStatus {
        guard let status = model.systemProxyStatus else {
            return WorkspaceStatus(title: "Unknown", systemImage: "questionmark.circle.fill", color: .secondary)
        }
        if status.matchesRequested {
            return WorkspaceStatus(title: "Enabled", systemImage: "checkmark.circle.fill", color: .green)
        }
        if status.enabled {
            return WorkspaceStatus(title: "Other Proxy", systemImage: "exclamationmark.circle.fill", color: .orange)
        }
        return WorkspaceStatus(title: "Disabled", systemImage: "xmark.circle.fill", color: .secondary)
    }

    private var primaryService: SystemProxyServiceStatus? {
        model.systemProxyStatus?.services.first
    }

    private var serviceText: String {
        guard let services = model.systemProxyStatus?.services, !services.isEmpty else {
            return "-"
        }
        return services.map(\.service).joined(separator: ", ")
    }
}
