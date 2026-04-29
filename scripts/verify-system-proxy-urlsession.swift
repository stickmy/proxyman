#!/usr/bin/env swift
import Darwin
import Foundation

struct CommandError: Error, CustomStringConvertible {
    let command: String
    let status: Int32
    let output: String

    var description: String {
        "\(command) failed with status \(status): \(output)"
    }
}

struct ProxySettings {
    var enabled: Bool
    var server: String
    var port: String
}

struct ServiceSnapshot {
    var service: String
    var http: ProxySettings
    var https: ProxySettings
    var bypassDomains: [String]
}

final class FakeProxyServer {
    private let fd: Int32
    private let lock = NSLock()
    private var stopped = false
    private(set) var lines: [String] = []
    let port: UInt16

    init() throws {
        let socketFd = socket(AF_INET, SOCK_STREAM, 0)
        guard socketFd >= 0 else {
            throw POSIXError(.init(rawValue: errno) ?? .EIO)
        }
        var shouldCloseSocket = true
        defer {
            if shouldCloseSocket {
                Darwin.close(socketFd)
            }
        }

        var yes: Int32 = 1
        setsockopt(socketFd, SOL_SOCKET, SO_REUSEADDR, &yes, socklen_t(MemoryLayout.size(ofValue: yes)))

        var address = sockaddr_in()
        address.sin_family = sa_family_t(AF_INET)
        address.sin_port = UInt16(0).bigEndian
        address.sin_addr.s_addr = inet_addr("127.0.0.1")

        let bindResult = withUnsafePointer(to: &address) { pointer in
            pointer.withMemoryRebound(to: sockaddr.self, capacity: 1) { socketAddress in
                Darwin.bind(socketFd, socketAddress, socklen_t(MemoryLayout<sockaddr_in>.size))
            }
        }
        guard bindResult == 0 else {
            throw POSIXError(.init(rawValue: errno) ?? .EIO)
        }
        guard listen(socketFd, 16) == 0 else {
            throw POSIXError(.init(rawValue: errno) ?? .EIO)
        }

        var boundAddress = sockaddr_in()
        var length = socklen_t(MemoryLayout<sockaddr_in>.size)
        let nameResult = withUnsafeMutablePointer(to: &boundAddress) { pointer in
            pointer.withMemoryRebound(to: sockaddr.self, capacity: 1) { socketAddress in
                getsockname(socketFd, socketAddress, &length)
            }
        }
        guard nameResult == 0 else {
            throw POSIXError(.init(rawValue: errno) ?? .EIO)
        }

        fd = socketFd
        port = UInt16(bigEndian: boundAddress.sin_port)
        shouldCloseSocket = false
    }

    func start() {
        DispatchQueue.global(qos: .userInitiated).async {
            while true {
                self.lock.lock()
                let shouldStop = self.stopped
                self.lock.unlock()
                if shouldStop {
                    return
                }

                let client = accept(self.fd, nil, nil)
                if client < 0 {
                    continue
                }
                self.handle(client: client)
            }
        }
    }

    func stop() {
        lock.lock()
        stopped = true
        lock.unlock()
        Darwin.close(fd)
    }

    func waitForLine(where predicate: (String) -> Bool, timeout: TimeInterval) -> String? {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            lock.lock()
            let match = lines.first(where: predicate)
            lock.unlock()
            if let match {
                return match
            }
            Thread.sleep(forTimeInterval: 0.05)
        }
        return nil
    }

    private func handle(client: Int32) {
        DispatchQueue.global(qos: .userInitiated).async {
            defer { Darwin.close(client) }
            var buffer = [UInt8](repeating: 0, count: 4096)
            let count = recv(client, &buffer, buffer.count, 0)
            guard count > 0 else {
                return
            }
            let data = Data(buffer.prefix(count))
            let text = String(decoding: data, as: UTF8.self)
            let line = text.components(separatedBy: "\r\n").first ?? text

            self.lock.lock()
            self.lines.append(line)
            self.lock.unlock()

            let response: String
            if line.hasPrefix("CONNECT ") {
                response = "HTTP/1.1 200 Connection Established\r\n\r\n"
            } else {
                response = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            }
            response.withCString { pointer in
                _ = Darwin.write(client, pointer, strlen(pointer))
            }
        }
    }
}

func run(_ executable: String, _ args: [String]) throws -> String {
    let process = Process()
    process.executableURL = URL(fileURLWithPath: executable)
    process.arguments = args
    let pipe = Pipe()
    process.standardOutput = pipe
    process.standardError = pipe
    try process.run()
    process.waitUntilExit()
    let data = pipe.fileHandleForReading.readDataToEndOfFile()
    let output = String(decoding: data, as: UTF8.self)
    guard process.terminationStatus == 0 else {
        throw CommandError(
            command: ([executable] + args).joined(separator: " "),
            status: process.terminationStatus,
            output: output
        )
    }
    return output
}

func parseProxySettings(_ output: String) -> ProxySettings {
    var values: [String: String] = [:]
    for line in output.split(separator: "\n") {
        let parts = line.split(separator: ":", maxSplits: 1).map {
            $0.trimmingCharacters(in: .whitespaces)
        }
        if parts.count == 2 {
            values[parts[0]] = parts[1]
        }
    }
    return ProxySettings(
        enabled: values["Enabled"]?.caseInsensitiveCompare("Yes") == .orderedSame,
        server: values["Server"] ?? "",
        port: values["Port"] ?? ""
    )
}

func parseServices(_ output: String) -> [String] {
    output
        .split(separator: "\n")
        .map { String($0).trimmingCharacters(in: .whitespacesAndNewlines) }
        .filter { !$0.isEmpty }
        .filter { !$0.hasPrefix("An asterisk") }
        .filter { !$0.hasPrefix("*") }
}

func parseBypassDomains(_ output: String) -> [String] {
    if output.contains("There aren't any bypass domains") {
        return []
    }
    return output
        .split(separator: "\n")
        .map { String($0).trimmingCharacters(in: .whitespacesAndNewlines) }
        .filter { !$0.isEmpty }
}

func snapshot(service: String) throws -> ServiceSnapshot {
    let http = parseProxySettings(try run("/usr/sbin/networksetup", ["-getwebproxy", service]))
    let https = parseProxySettings(try run("/usr/sbin/networksetup", ["-getsecurewebproxy", service]))
    let bypass = parseBypassDomains(
        try run("/usr/sbin/networksetup", ["-getproxybypassdomains", service])
    )
    return ServiceSnapshot(service: service, http: http, https: https, bypassDomains: bypass)
}

func setProxy(service: String, port: UInt16) throws {
    let portText = String(port)
    _ = try run("/usr/sbin/networksetup", ["-setwebproxy", service, "127.0.0.1", portText])
    _ = try run("/usr/sbin/networksetup", ["-setsecurewebproxy", service, "127.0.0.1", portText])
    _ = try run("/usr/sbin/networksetup", ["-setwebproxystate", service, "on"])
    _ = try run("/usr/sbin/networksetup", ["-setsecurewebproxystate", service, "on"])
    _ = try run("/usr/sbin/networksetup", ["-setproxybypassdomains", service, "Empty"])
}

func restore(_ snapshot: ServiceSnapshot) {
    do {
        if snapshot.http.enabled {
            _ = try run(
                "/usr/sbin/networksetup",
                ["-setwebproxy", snapshot.service, snapshot.http.server, snapshot.http.port]
            )
            _ = try run("/usr/sbin/networksetup", ["-setwebproxystate", snapshot.service, "on"])
        } else {
            _ = try run("/usr/sbin/networksetup", ["-setwebproxystate", snapshot.service, "off"])
        }

        if snapshot.https.enabled {
            _ = try run(
                "/usr/sbin/networksetup",
                ["-setsecurewebproxy", snapshot.service, snapshot.https.server, snapshot.https.port]
            )
            _ = try run("/usr/sbin/networksetup", ["-setsecurewebproxystate", snapshot.service, "on"])
        } else {
            _ = try run("/usr/sbin/networksetup", ["-setsecurewebproxystate", snapshot.service, "off"])
        }

        let bypassArgs = snapshot.bypassDomains.isEmpty ? ["Empty"] : snapshot.bypassDomains
        _ = try run(
            "/usr/sbin/networksetup",
            ["-setproxybypassdomains", snapshot.service] + bypassArgs
        )
    } catch {
        fputs("restore failed for \(snapshot.service): \(error)\n", stderr)
    }
}

func request(_ url: URL, timeout: TimeInterval = 5) {
    let config = URLSessionConfiguration.ephemeral
    config.timeoutIntervalForRequest = timeout
    config.timeoutIntervalForResource = timeout
    config.requestCachePolicy = .reloadIgnoringLocalCacheData
    let session = URLSession(configuration: config)
    let semaphore = DispatchSemaphore(value: 0)
    session.dataTask(with: url) { _, _, _ in
        semaphore.signal()
    }.resume()
    _ = semaphore.wait(timeout: .now() + timeout + 1)
    session.invalidateAndCancel()
}

func selectedService() throws -> String {
    let args = CommandLine.arguments
    if let index = args.firstIndex(of: "--service"), args.indices.contains(index + 1) {
        return args[index + 1]
    }
    let services = parseServices(try run("/usr/sbin/networksetup", ["-listallnetworkservices"]))
    guard let first = services.first else {
        throw NSError(domain: "verify-system-proxy", code: 1, userInfo: [
            NSLocalizedDescriptionKey: "No enabled network service found"
        ])
    }
    return first
}

func main() throws {
    if CommandLine.arguments.contains("--help") {
        print("Usage: verify-system-proxy-urlsession.swift [--service <network service>]")
        return
    }

    let service = try selectedService()
    let fakeProxy = try FakeProxyServer()
    fakeProxy.start()
    let before = try snapshot(service: service)
    defer {
        restore(before)
        fakeProxy.stop()
    }

    try setProxy(service: service, port: fakeProxy.port)
    let scutil = try run("/usr/sbin/scutil", ["--proxy"])
    guard scutil.contains("HTTPProxy : 127.0.0.1"),
          scutil.contains("HTTPSProxy : 127.0.0.1") else {
        throw NSError(domain: "verify-system-proxy", code: 2, userInfo: [
            NSLocalizedDescriptionKey: "scutil --proxy did not report the fake proxy"
        ])
    }

    request(URL(string: "http://example.com/proxyman-system-proxy-http")!)
    guard let httpLine = fakeProxy.waitForLine(
        where: { $0.hasPrefix("GET http://example.com/proxyman-system-proxy-http") },
        timeout: 5
    ) else {
        throw NSError(domain: "verify-system-proxy", code: 3, userInfo: [
            NSLocalizedDescriptionKey: "URLSession HTTP request did not use absolute-form proxy request"
        ])
    }

    request(URL(string: "https://example.com/proxyman-system-proxy-https")!)
    guard let connectLine = fakeProxy.waitForLine(
        where: { $0.hasPrefix("CONNECT example.com:443") },
        timeout: 5
    ) else {
        throw NSError(domain: "verify-system-proxy", code: 4, userInfo: [
            NSLocalizedDescriptionKey: "URLSession HTTPS request did not use CONNECT through proxy"
        ])
    }

    print("Verified \(service)")
    print(httpLine)
    print(connectLine)
}

do {
    try main()
} catch {
    fputs("\(error)\n", stderr)
    exit(1)
}
