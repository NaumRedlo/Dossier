import AppKit
import CoreGraphics

func appKitRect(_ rect: CGRect, primaryHeight: CGFloat) -> CGRect {
    CGRect(x: rect.minX, y: primaryHeight - rect.maxY, width: rect.width, height: rect.height)
}

struct Plate: Decodable { let x: Int; let y: Int; let width: Int; let height: Int; let png: String }
struct Frame: Decodable { let id: UInt64; let plates: [Plate]; let status: String; let map: String? }
final class Canvas: NSView { override var isFlipped: Bool { true } }
final class Panel: NSPanel {
    override var canBecomeKey: Bool { false }
    override var canBecomeMain: Bool { false }
}

func gameWindow(pid: pid_t) -> CGRect? {
    guard let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] else { return nil }
    return windows.compactMap { item -> CGRect? in
        guard (item[kCGWindowOwnerPID as String] as? Int) == Int(pid),
              (item[kCGWindowLayer as String] as? Int) == 0,
              let bounds = item[kCGWindowBounds as String] as? [String: Any],
              let rect = CGRect(dictionaryRepresentation: bounds as CFDictionary), rect.width >= 320, rect.height >= 240 else { return nil }
        return rect
    }.max(by: { $0.width * $0.height < $1.width * $1.height })
}

final class Overlay: NSObject, NSApplicationDelegate {
    let renderer: String
    let preview: Bool
    var demo: Bool { preview || CommandLine.arguments.contains("--demo") }
    let lazerSocket = LazerSocket()
    let liveCard = NSView()
    let liveStats = NSTextField(wrappingLabelWithString: "")
    let liveTitle = NSTextField(labelWithString: "")
    let liveMap = NSTextField(wrappingLabelWithString: "")
    var lastLiveRender = Date.distantPast
    var gamePID: pid_t = 0
    let smoke = CommandLine.arguments.contains("--smoke") || CommandLine.arguments.contains("--smoke-live")
    let panel = Panel(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
    let canvas = Canvas()
    let child = Process()
    let input = Pipe()
    let output = Pipe()
    var timer: Timer?
    var status: NSStatusItem?
    var previewWindow: NSWindow?
    var scene = "play"
    var nextID: UInt64 = 0
    var pending: (id: UInt64, rect: CGRect, scale: CGFloat, at: Date, pid: pid_t)?
    var target: CGRect?
    var scale: CGFloat = 1
    var images: [NSImageView] = []
    var diagnostic = ""
    var statusLine: NSMenuItem?
    var label = NSTextField(labelWithString: "DEMO · ТЕСТОВЫЕ ДАННЫЕ")

    init(renderer: String, preview: Bool) { self.renderer = renderer; self.preview = preview }
    func report(_ message: String) {
        guard diagnostic != message else { return }
        diagnostic = message
        fputs("HUD Demo: \(message)\n", stderr)
        statusLine?.title = message
    }
    func applicationDidFinishLaunching(_ notification: Notification) {
        report("Запущен. Переключитесь в окно lazer.")
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = false
        panel.ignoresMouseEvents = true
        panel.hidesOnDeactivate = false
        panel.isFloatingPanel = true
        panel.level = .floating
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .ignoresCycle]
        if #available(macOS 13.0, *) { panel.collectionBehavior.insert(.canJoinAllApplications) }
        panel.contentView = canvas
        label.textColor = .white
        label.backgroundColor = NSColor.systemRed.withAlphaComponent(0.85)
        label.drawsBackground = true
        label.font = .boldSystemFont(ofSize: 12)
        label.alignment = .center
        label.isHidden = !demo
        canvas.addSubview(label)
        liveCard.wantsLayer = true
        liveCard.layer?.backgroundColor = NSColor(calibratedWhite: 0.08, alpha: 0.94).cgColor
        liveCard.layer?.cornerRadius = 12
        liveCard.layer?.borderWidth = 1
        liveCard.layer?.borderColor = NSColor.systemRed.withAlphaComponent(0.7).cgColor
        liveTitle.font = .boldSystemFont(ofSize: 13)
        liveTitle.textColor = .white
        liveMap.font = .systemFont(ofSize: 12)
        liveMap.textColor = NSColor(calibratedWhite: 0.8, alpha: 1)
        liveMap.maximumNumberOfLines = 3
        liveMap.lineBreakMode = .byTruncatingTail
        liveStats.font = .systemFont(ofSize: 11)
        liveStats.textColor = NSColor(calibratedWhite: 0.75, alpha: 1)
        liveStats.maximumNumberOfLines = 2
        liveCard.addSubview(liveTitle); liveCard.addSubview(liveMap); liveCard.addSubview(liveStats)
        liveCard.isHidden = demo
        canvas.addSubview(liveCard)
        let item = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        item.button?.title = demo ? "HUD Demo" : "Dossier HUD"
        let menu = NSMenu()
        let state = NSMenuItem(title: diagnostic, action: nil, keyEquivalent: "")
        menu.addItem(state); menu.addItem(.separator()); statusLine = state
        for name in (demo ? ["menu", "select", "play", "rest", "results", "hold", "sent"] : []) {
            let choice = NSMenuItem(title: name, action: #selector(selectScene(_:)), keyEquivalent: "")
            choice.target = self; choice.representedObject = name; menu.addItem(choice)
        }
        menu.addItem(.separator())
        let quit = NSMenuItem(title: "Закрыть оверлей", action: #selector(quit), keyEquivalent: "")
        quit.target = self; menu.addItem(quit); item.menu = menu; status = item
        if preview {
            let window = NSWindow(contentRect: NSRect(x: 160, y: 160, width: 1100, height: 650), styleMask: [.titled, .closable, .resizable, .miniaturizable], backing: .buffered, defer: false)
            window.title = "Dossier HUD — тестовое окно"
            window.backgroundColor = NSColor(calibratedRed: 0.06, green: 0.04, blue: 0.08, alpha: 1)
            window.isReleasedWhenClosed = false
            window.makeKeyAndOrderFront(nil)
            previewWindow = window
            NSApp.activate(ignoringOtherApps: true)
        }
        child.executableURL = URL(fileURLWithPath: renderer)
        child.standardInput = input; child.standardOutput = output; child.standardError = FileHandle.standardError
        child.terminationHandler = { process in
            fputs("HUD renderer exited: \(process.terminationStatus)\n", stderr)
            DispatchQueue.main.async { NSApp.terminate(nil) }
        }
        do { try child.run() } catch { fputs("HUD renderer could not start: \(error)\n", stderr); NSApp.terminate(nil); return }
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self = self else { return }
            var buffer = Data()
            while true {
                let bytes = self.output.fileHandleForReading.availableData
                if bytes.isEmpty { break }
                buffer.append(bytes)
                if buffer.count > 8 * 1024 * 1024 { break }
                while let newline = buffer.firstIndex(of: 10) {
                    let line = Data(buffer[..<newline]); buffer.removeSubrange(...newline)
                    let frame: Frame
                    do { frame = try JSONDecoder().decode(Frame.self, from: line) } catch {
                        fputs("Invalid HUD response: \(error)\n", stderr)
                        DispatchQueue.main.async { NSApp.terminate(nil) }; return
                    }
                    DispatchQueue.main.async { self.receive(frame) }
                }
            }
            DispatchQueue.main.async { NSApp.terminate(nil) }
        }
        if smoke {
            DispatchQueue.main.asyncAfter(deadline: .now() + 10) {
                fputs("Native window smoke check timed out\n", stderr)
                if self.child.isRunning { self.child.terminate() }
                exit(2)
            }
        }
        timer = Timer(timeInterval: 0.05, repeats: true) { [weak self] _ in self?.tick() }
        RunLoop.main.add(timer!, forMode: .common)
    }
    @objc func selectScene(_ sender: NSMenuItem) { scene = sender.representedObject as? String ?? "play" }
    @objc func quit() { NSApp.terminate(nil) }
    func tick() {
        let rect: CGRect?
        var startedMS: UInt64 = 0
        gamePID = 0
        if let window = previewWindow {
            rect = window.isVisible && !window.isMiniaturized && (NSApp.isActive || smoke) ? window.convertToScreen(window.contentView!.bounds) : nil
        } else if let front = NSWorkspace.shared.frontmostApplication, front.bundleIdentifier == "sh.ppy.osu.lazer", let game = gameWindow(pid: front.processIdentifier) {
            gamePID = front.processIdentifier
            startedMS = UInt64(max(0, (front.launchDate?.timeIntervalSince1970 ?? 0) * 1000))
            rect = appKitRect(game, primaryHeight: NSScreen.screens.first?.frame.height ?? 0)
        } else {
            rect = nil
            let games = NSWorkspace.shared.runningApplications.filter { $0.bundleIdentifier == "sh.ppy.osu.lazer" }
            if games.isEmpty { report("lazer не запущен") }
            else if NSWorkspace.shared.frontmostApplication?.bundleIdentifier != "sh.ppy.osu.lazer" { report("Ожидание: переключитесь в окно lazer") }
            else { report("lazer активен, но видимое окно не найдено (свёрнуто или в другом Space)") }
        }
        if !demo {
            let running = NSWorkspace.shared.runningApplications.first { $0.bundleIdentifier == "sh.ppy.osu.lazer" }
            lazerSocket.tick(pid: running?.processIdentifier ?? 0)
        }
        target = rect
        guard let rect = rect else { panel.orderOut(nil); return }
        let screen = NSScreen.screens.max { a, b in
            let x = a.frame.intersection(rect); let y = b.frame.intersection(rect)
            return (x.isNull ? 0 : x.width * x.height) < (y.isNull ? 0 : y.width * y.height)
        }
        scale = screen?.backingScaleFactor ?? 1
        if let pending = pending {
            if Date().timeIntervalSince(pending.at) > 2 { panel.orderOut(nil); fputs("HUD renderer timed out\n", stderr); NSApp.terminate(nil) }
            return
        }
        if !panel.isVisible { report("Окно найдено, отрисовка плашек") }
        if !demo && panel.isVisible && panel.frame == rect && Date().timeIntervalSince(lastLiveRender) < 0.25 { return }
        lastLiveRender = Date()
        nextID += 1
        var request: [String: Any] = ["id": nextID, "width": Int((rect.width * scale).rounded()), "height": Int((rect.height * scale).rounded()), "pid": gamePID, "started_ms": startedMS]
        if demo { request["scene"] = scene }
        guard let json = try? JSONSerialization.data(withJSONObject: request) else { return }
        pending = (nextID, rect, scale, Date(), gamePID)
        do { try input.fileHandleForWriting.write(contentsOf: json + Data([10])) }
        catch { NSApp.terminate(nil) }
    }
    func receive(_ frame: Frame) {
        guard let request = pending, request.id == frame.id else { return }
        pending = nil
        guard target == request.rect, scale == request.scale, gamePID == request.pid else { return }
        // Recheck focus when the renderer finishes, not only when the request starts.
        guard smoke || (preview ? NSApp.isActive : (NSWorkspace.shared.frontmostApplication?.bundleIdentifier == "sh.ppy.osu.lazer" && NSWorkspace.shared.frontmostApplication?.processIdentifier == request.pid)) else { panel.orderOut(nil); return }
        let rect = request.rect
        panel.setFrame(rect, display: false)
        canvas.frame = NSRect(origin: .zero, size: rect.size)
        while images.count < frame.plates.count { let image = NSImageView(); image.imageScaling = .scaleAxesIndependently; canvas.addSubview(image, positioned: .below, relativeTo: label); images.append(image) }
        for (index, image) in images.enumerated() {
            guard index < frame.plates.count else { image.isHidden = true; continue }
            let plate = frame.plates[index]
            guard let data = Data(base64Encoded: plate.png), let picture = NSImage(data: data) else { image.isHidden = true; continue }
            image.image = picture
            image.frame = NSRect(x: CGFloat(plate.x) / scale, y: CGFloat(plate.y) / scale, width: CGFloat(plate.width) / scale, height: CGFloat(plate.height) / scale)
            image.isHidden = false
        }
        label.frame = NSRect(x: max(0, (rect.width - 250) / 2), y: 12, width: min(250, rect.width), height: 22)
        if !demo {
            let width = min(380, max(180, rect.width - 32))
            liveCard.frame = NSRect(x: max(0, rect.width - width - 16), y: max(0, rect.height - 154), width: width, height: 138)
            liveTitle.frame = NSRect(x: 14, y: 108, width: width - 28, height: 18)
            liveMap.frame = NSRect(x: 14, y: 54, width: width - 28, height: 48)
            liveTitle.stringValue = "lazer · " + frame.status
            liveMap.stringValue = lazerSocket.event?.beatmap.title ?? frame.map ?? "Данные текущей карты пока недоступны"
            liveStats.frame = NSRect(x: 14, y: 12, width: width - 28, height: 36)
            if let event = lazerSocket.event {
                let mods = event.mods.isEmpty ? "NM" : event.mods.map { $0.acronym }.joined(separator: " ")
                liveStats.stringValue = mods + " · " + event.beatmap.summary
            } else { liveStats.stringValue = lazerSocket.diagnostic }
        }
        // Reordering on every frame can interfere with fullscreen window ordering.
        if !panel.isVisible { panel.orderFrontRegardless() }
        report(demo ? "Плашки показаны · тестовые данные" : lazerSocket.diagnostic + " · " + frame.status)
        if smoke {
            precondition(panel.ignoresMouseEvents && !panel.canBecomeKey && !panel.canBecomeMain)
            precondition(panel.isVisible)
            precondition(demo ? !images.isEmpty : (!liveCard.isHidden && images.isEmpty))
            precondition(panel.frame == request.rect)
            if let bitmap = canvas.bitmapImageRepForCachingDisplay(in: canvas.bounds) {
                canvas.cacheDisplay(in: canvas.bounds, to: bitmap)
                try? bitmap.representation(using: .png, properties: [:])?.write(to: URL(fileURLWithPath: "/private/tmp/dossier-macos-hud-preview.png"))
            }
            print("Native window smoke check passed: \(images.count) plates, scale=\(scale), key=\(panel.isKeyWindow), status=\(frame.status)")
            NSApp.terminate(nil)
        }
    }
    func applicationWillTerminate(_ notification: Notification) {
        timer?.invalidate(); lazerSocket.stop(); panel.orderOut(nil)
        try? input.fileHandleForWriting.close()
        if child.isRunning { child.terminate() }
    }
}

let args = CommandLine.arguments
if args.contains("--self-test") {
    precondition(appKitRect(CGRect(x: 100, y: 40, width: 800, height: 600), primaryHeight: 900) == CGRect(x: 100, y: 260, width: 800, height: 600))
    precondition(appKitRect(CGRect(x: -1000, y: -800, width: 800, height: 600), primaryHeight: 900) == CGRect(x: -1000, y: 1100, width: 800, height: 600))
    print("Coordinate checks passed")
    testLazerMessages()
} else if args.contains("--api-test"), let port = UInt16(ProcessInfo.processInfo.environment["DOSSIER_TEST_PORT"] ?? "") {
    let socket = LazerSocket(port: port)
    let deadline = Date().addingTimeInterval(5)
    var seen = false
    var cleared = false
    while Date() < deadline {
        socket.tick(pid: 1)
        RunLoop.current.run(until: Date().addingTimeInterval(0.02))
        if let event = socket.event { precondition(event.beatmap.max_combo == 1234); seen = true }
        if seen && socket.event == nil { cleared = true; break }
    }
    socket.stop()
    precondition(seen && cleared, "Expected a map event followed by clearing on disconnect")
    print("Lazer API connection/disconnect checks passed")
} else if args.contains("--probe") {
    let apps = NSWorkspace.shared.runningApplications.filter { $0.bundleIdentifier == "sh.ppy.osu.lazer" }
    for app in apps { print("lazer pid=\(app.processIdentifier), window=\(String(describing: gameWindow(pid: app.processIdentifier)))") }
    if apps.isEmpty { print("lazer is not running") }
    print("front=\(NSWorkspace.shared.frontmostApplication?.bundleIdentifier ?? "unknown")")
    let rows = CGWindowListCopyWindowInfo([.optionAll, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
    print("window rows=\(rows.count)")
    for row in rows where apps.contains(where: { Int($0.processIdentifier) == (row[kCGWindowOwnerPID as String] as? Int) }) {
        print("lazer window layer=\(String(describing: row[kCGWindowLayer as String])), bounds=\(String(describing: row[kCGWindowBounds as String])), onscreen=\(String(describing: row[kCGWindowIsOnscreen as String]))")
    }
} else {
    let renderer = args.dropFirst().first(where: { !$0.hasPrefix("--") }) ?? Bundle.main.path(forResource: "hud-preview", ofType: nil) ?? ""
    guard FileManager.default.isExecutableFile(atPath: renderer) else { fputs("HUD renderer is missing. Run tools/macos-overlay/run.sh to rebuild.\n", stderr); exit(1) }
    let application = NSApplication.shared
    application.setActivationPolicy(.accessory)
    let delegate = Overlay(renderer: renderer, preview: args.contains("--preview") || args.contains("--smoke"))
    application.delegate = delegate
    application.run()
}
