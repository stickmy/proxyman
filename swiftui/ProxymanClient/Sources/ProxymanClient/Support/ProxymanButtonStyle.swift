import AppKit
import SwiftUI

enum ProxymanTheme {
    enum Button {
        static let cornerRadius: CGFloat = 8
        static let hoverBackground = Color(red: 0xF2 / 255, green: 0xF2 / 255, blue: 0xF0 / 255)
        static let border = Color(nsColor: .separatorColor).opacity(0.36)
        static let disabledBorder = Color(nsColor: .separatorColor).opacity(0.20)
    }
}

struct ProxymanButtonStyle: ButtonStyle {
    enum Size {
        case regular
        case icon

        var height: CGFloat {
            switch self {
            case .regular:
                28
            case .icon:
                24
            }
        }

        var horizontalPadding: CGFloat {
            switch self {
            case .regular:
                10
            case .icon:
                0
            }
        }
    }

    var size: Size = .regular

    func makeBody(configuration: Configuration) -> some View {
        ProxymanButtonBody(configuration: configuration, size: size)
    }
}

private struct ProxymanButtonBody: View {
    let configuration: ProxymanButtonStyle.Configuration
    let size: ProxymanButtonStyle.Size

    @Environment(\.isEnabled) private var isEnabled
    @State private var isHovered = false

    var body: some View {
        configuration.label
            .lineLimit(1)
            .padding(.horizontal, size.horizontalPadding)
            .frame(width: size == .icon ? size.height : nil, height: size.height)
            .background(background, in: RoundedRectangle(cornerRadius: ProxymanTheme.Button.cornerRadius, style: .continuous))
            .overlay {
                RoundedRectangle(cornerRadius: ProxymanTheme.Button.cornerRadius, style: .continuous)
                    .stroke(isEnabled ? ProxymanTheme.Button.border : ProxymanTheme.Button.disabledBorder, lineWidth: 1)
            }
            .contentShape(RoundedRectangle(cornerRadius: ProxymanTheme.Button.cornerRadius, style: .continuous))
            .opacity(isEnabled ? 1 : 0.46)
            .onHover { isHovered = $0 }
    }

    private var background: Color {
        guard isEnabled else { return .clear }
        if configuration.isPressed {
            return ProxymanTheme.Button.hoverBackground.opacity(0.78)
        }
        if isHovered {
            return ProxymanTheme.Button.hoverBackground
        }
        return .clear
    }
}

extension ButtonStyle where Self == ProxymanButtonStyle {
    static var proxymanAction: ProxymanButtonStyle {
        ProxymanButtonStyle(size: .regular)
    }

    static var proxymanIcon: ProxymanButtonStyle {
        ProxymanButtonStyle(size: .icon)
    }
}
