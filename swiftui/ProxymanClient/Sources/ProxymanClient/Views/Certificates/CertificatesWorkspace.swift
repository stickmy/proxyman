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

                HStack(spacing: 8) {
                    Button {
                        Task { await model.refreshCaStatus() }
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
}
