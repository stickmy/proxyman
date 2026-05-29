import SwiftUI

@main
struct ProxymanClientApp: App {
    @StateObject private var model = AppModel(coreClient: Self.makeCoreClient())

    var body: some Scene {
        WindowGroup("Proxyman") {
            ContentView()
                .environmentObject(model)
                .frame(minWidth: 980, minHeight: 620)
        }
        .windowStyle(.hiddenTitleBar)
        .defaultSize(width: 1120, height: 720)
        .commands {
            CommandGroup(after: .appInfo) {
                Button("Start Proxy") {
                    Task { await model.startProxy() }
                }
                .keyboardShortcut("r", modifiers: [.command])

                Button("Stop Proxy") {
                    Task { await model.stopProxy() }
                }
                .keyboardShortcut(".", modifiers: [.command])
            }
        }
    }

    private static func makeCoreClient() -> CoreClient {
        let environment = ProcessInfo.processInfo.environment
        if let commandSocket = environment["PROXYMAN_COMMAND_SOCKET"] {
            return UnixSocketCoreClient(
                configuration: UnixSocketConfiguration(
                    commandSocketPath: commandSocket,
                    eventSocketPath: environment["PROXYMAN_EVENT_SOCKET"] ?? ""
                )
            )
        }

        do {
            let launch = try SidecarLauncher.start()
            return UnixSocketCoreClient(
                configuration: launch.configuration,
                sidecarProcess: launch.process
            )
        } catch {
            return MockCoreClient()
        }
    }
}
