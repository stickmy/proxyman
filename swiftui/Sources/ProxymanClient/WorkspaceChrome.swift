import AppKit
import SwiftUI

struct WorkspaceStatus {
    var title: String
    var systemImage: String
    var color: Color
}

struct WorkspaceHeader: View {
    var title: String
    var systemImage: String
    var status: WorkspaceStatus

    var body: some View {
        HStack(spacing: 8) {
            Label(title, systemImage: systemImage)
                .font(.system(size: 13, weight: .semibold))

            Spacer()

            StatusBadge(status: status)
        }
        .padding(.horizontal, 12)
        .frame(height: 48)
        .background(.bar)
    }
}

struct SettingsRow: View {
    var title: String
    var value: String

    var body: some View {
        Grid(alignment: .leading, horizontalSpacing: 18, verticalSpacing: 0) {
            GridRow {
                Text(title)
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(.secondary)
                    .frame(width: 96, alignment: .leading)

                Text(value)
                    .font(.system(size: 12, weight: .semibold))
                    .textSelection(.enabled)
            }
        }
    }
}

private struct StatusBadge: View {
    var status: WorkspaceStatus

    var body: some View {
        Label(status.title, systemImage: status.systemImage)
            .font(.system(size: 11, weight: .semibold))
            .foregroundStyle(status.color)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(.quaternary.opacity(0.35), in: Capsule())
    }
}
