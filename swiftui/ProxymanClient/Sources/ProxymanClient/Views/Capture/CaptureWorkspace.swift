import AppKit
import SwiftUI

struct CaptureWorkspace: View {
    var body: some View {
        HStack(spacing: 0) {
            ExchangeList()
                .frame(minWidth: 420)

            Rectangle()
                .fill(.separator.opacity(0.18))
                .frame(width: 1)

            ExchangeDetail()
                .frame(minWidth: 360)
        }
    }
}

private struct ExchangeList: View {
    @EnvironmentObject private var model: AppModel

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Exchanges")
                    .font(.system(size: 13, weight: .semibold))

                Spacer()

                Text("\(model.exchanges.count)")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(.secondary)

                Button {
                    Task { await model.clearSession() }
                } label: {
                    Label("Clear", systemImage: "trash")
                }
                .labelStyle(.iconOnly)
                .buttonStyle(.proxymanIcon)
                .foregroundStyle(.secondary)
                .help("Clear captured exchanges")
                .clickableCursor()
            }
            .padding(.horizontal, 12)
            .frame(height: 42)

            List(model.exchanges, selection: $model.selectedExchangeID) { exchange in
                ExchangeRow(exchange: exchange)
                    .tag(exchange.id)
                    .contentShape(Rectangle())
                    .clickableCursor()
            }
            .listStyle(.plain)
            .scrollContentBackground(.hidden)
        }
        .background(Color(nsColor: .controlBackgroundColor).opacity(0.55))
    }
}

private struct ExchangeRow: View {
    var exchange: ExchangeSummary

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            HStack(spacing: 8) {
                Text(exchange.method)
                    .font(.system(size: 11, weight: .bold, design: .monospaced))
                    .foregroundStyle(methodColor)
                    .frame(width: 44, alignment: .leading)

                Text(exchange.host)
                    .font(.system(size: 12, weight: .semibold))
                    .lineLimit(1)

                Spacer()

                Text(exchange.statusText)
                    .font(.system(size: 11, weight: .semibold, design: .monospaced))
                    .foregroundStyle(statusColor)
            }

            Text(exchange.path)
                .font(.system(size: 12, design: .monospaced))
                .foregroundStyle(.secondary)
                .lineLimit(1)

            HStack(spacing: 8) {
                Text(exchange.durationText)
                Text("\(exchange.responseBytes) B")
            }
            .font(.system(size: 11))
            .foregroundStyle(.tertiary)
        }
        .padding(.vertical, 5)
    }

    private var methodColor: Color {
        switch exchange.method {
        case "POST", "PUT", "PATCH":
            .orange
        case "DELETE":
            .red
        default:
            .blue
        }
    }

    private var statusColor: Color {
        guard let status = exchange.status else { return .secondary }
        switch status {
        case 200..<300:
            return .green
        case 300..<400:
            return .blue
        case 400..<500:
            return .orange
        default:
            return .red
        }
    }
}

private struct ExchangeDetail: View {
    @EnvironmentObject private var model: AppModel

    var body: some View {
        VStack(spacing: 0) {
            header

            if let exchange = model.selectedExchange {
                ScrollView {
                    VStack(alignment: .leading, spacing: 16) {
                        PreviewSection(title: "Request", text: exchange.requestPreview)
                        PreviewSection(title: "Response", text: exchange.responsePreview)
                    }
                    .padding(14)
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
            } else {
                ContentUnavailableView("No Exchange Selected", systemImage: "sidebar.leading")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .background(Color(nsColor: .textBackgroundColor))
        .task(id: model.selectedExchangeID) {
            await model.loadSelectedExchangeBodies()
        }
    }

    private var header: some View {
        HStack(spacing: 8) {
            if let exchange = model.selectedExchange {
                Text(exchange.method)
                    .font(.system(size: 12, weight: .bold, design: .monospaced))

                Text(exchange.host)
                    .font(.system(size: 13, weight: .semibold))
                    .lineLimit(1)

                Spacer()

                Button {
                    Task { await model.replaySelectedExchange() }
                } label: {
                    Label("Replay", systemImage: "paperplane")
                }
                .buttonStyle(.proxymanAction)
                .help("Replay request")
                .disabled(model.isBusy)
                .clickableCursor(isEnabled: !model.isBusy)
            } else {
                Text("Detail")
                    .font(.system(size: 13, weight: .semibold))
                Spacer()
            }
        }
        .padding(.horizontal, 12)
        .frame(height: 42)
    }
}

private struct PreviewSection: View {
    var title: String
    var text: String

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            Text(title)
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(.secondary)

            ScrollView(.horizontal) {
                Text(text)
                    .font(.system(size: 12, design: .monospaced))
                    .textSelection(.enabled)
                    .padding(10)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .background(Color(nsColor: .controlBackgroundColor).opacity(0.65), in: RoundedRectangle(cornerRadius: 6))
        }
    }
}
