import AppKit
import SwiftUI

extension View {
    func clickableCursor(isEnabled: Bool = true) -> some View {
        modifier(ClickableCursorModifier(isEnabled: isEnabled))
    }
}

private struct ClickableCursorModifier: ViewModifier {
    var isEnabled: Bool
    @State private var isHovering = false

    func body(content: Content) -> some View {
        content
            .onHover { hovering in
                updateCursor(hovering: hovering)
            }
            .onChange(of: isEnabled) { _, enabled in
                if !enabled {
                    popCursorIfNeeded()
                }
            }
            .onDisappear {
                popCursorIfNeeded()
            }
    }

    private func updateCursor(hovering: Bool) {
        if hovering && isEnabled && !isHovering {
            NSCursor.pointingHand.push()
            isHovering = true
        } else if (!hovering || !isEnabled) && isHovering {
            popCursorIfNeeded()
        }
    }

    private func popCursorIfNeeded() {
        guard isHovering else { return }
        NSCursor.pop()
        isHovering = false
    }
}
