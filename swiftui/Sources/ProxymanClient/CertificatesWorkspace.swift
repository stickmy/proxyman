import AppKit
import SwiftUI

struct CertificatesWorkspace: View {
    @EnvironmentObject private var model: AppModel

    var body: some View {
        VStack(spacing: 0) {
            WorkspaceHeader(
                title: "Certificates",
                systemImage: "checkmark.seal",
                status: certificateStatus
            )

            VStack(alignment: .leading, spacing: 18) {
                SettingsRow(title: "Trust", value: certificateStatus.title)
                SettingsRow(title: "Authority", value: "Proxyman Local CA")
                SettingsToggleRow(title: "Upstream TLS", value: upstreamTlsStatusText) {
                    UpstreamTlsVerificationPill(
                        ignoresVerification: model.ignoreUpstreamTlsVerification == true,
                        isInteractive: !model.isBusy,
                        action: {
                            let nextValue = !(model.ignoreUpstreamTlsVerification ?? false)
                            Task { await model.setIgnoreUpstreamTlsVerification(nextValue) }
                        }
                    )
                }

                HStack(spacing: 8) {
                    Button {
                        Task {
                            await model.refreshCaStatus()
                            await model.refreshUpstreamTlsStatus()
                        }
                    } label: {
                        Label("Refresh", systemImage: "arrow.clockwise")
                    }
                    .buttonStyle(.proxymanAction)
                    .disabled(model.isBusy)
                    .clickableCursor(isEnabled: !model.isBusy)

                    Button {
                        Task { await model.installCa() }
                    } label: {
                        Label(certificateInstallTitle, systemImage: "checkmark.seal")
                    }
                    .buttonStyle(.proxymanAction)
                    .disabled(model.isBusy)
                    .clickableCursor(isEnabled: !model.isBusy)
                }
            }
            .padding(20)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
        .background(Color(nsColor: .textBackgroundColor))
        .task {
            await model.refreshCaStatus()
            await model.refreshUpstreamTlsStatus()
        }
    }

    private var certificateInstallTitle: String {
        model.caInstalled == true ? "Reinstall" : "Install"
    }

    private var certificateStatus: WorkspaceStatus {
        switch model.caInstalled {
        case true:
            WorkspaceStatus(title: "Trusted", systemImage: "checkmark.circle.fill", color: .green)
        case false:
            WorkspaceStatus(title: "Missing", systemImage: "xmark.circle.fill", color: .red)
        case nil:
            WorkspaceStatus(title: "Unknown", systemImage: "questionmark.circle.fill", color: .secondary)
        }
    }

    private var upstreamTlsStatusText: String {
        switch model.ignoreUpstreamTlsVerification {
        case true:
            "Ignored"
        case false:
            "System Trust"
        case nil:
            "Unknown"
        }
    }
}

private struct SettingsToggleRow<Control: View>: View {
    var title: String
    var value: String
    @ViewBuilder var control: () -> Control

    var body: some View {
        Grid(alignment: .leading, horizontalSpacing: 18, verticalSpacing: 0) {
            GridRow {
                Text(title)
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(.secondary)
                    .frame(width: 96, alignment: .leading)

                HStack(spacing: 10) {
                    Text(value)
                        .font(.system(size: 12, weight: .semibold))
                        .frame(width: 92, alignment: .leading)
                    control()
                }
            }
        }
    }
}

private struct UpstreamTlsVerificationPill: View {
    var ignoresVerification: Bool
    var isInteractive: Bool
    var action: () -> Void
    @State private var isHovered = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: 5) {
                Circle()
                    .fill(ignoresVerification ? Color.orange : Color.green)
                    .frame(width: 6, height: 6)

                Text(ignoresVerification ? "Ignore" : "Verify")
                    .font(.system(size: 11, weight: .semibold))
            }
            .foregroundStyle(ignoresVerification ? Color.orange : Color.primary)
            .padding(.horizontal, 9)
            .frame(height: 22)
            .background(background, in: Capsule())
            .overlay {
                Capsule()
                    .stroke(Color(nsColor: .separatorColor).opacity(0.28), lineWidth: 1)
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
        if ignoresVerification {
            return Color.orange.opacity(0.10)
        }
        return Color.green.opacity(0.08)
    }
}
