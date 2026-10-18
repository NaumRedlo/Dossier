import Foundation

// Verified against the shipped 2026.1005.1.0 client. These are map properties,
// never current-play combo/accuracy/PP. Unknown message types are ignored.
struct LazerMap: Decodable {
    struct Metadata: Decodable { let title: String; let artist: String }
    let metadata: Metadata
    let difficulty_name: String
    let star_rating: Double
    let maximum_pp: Double
    let max_combo: Int
    let bpm: Double
    var title: String { "\(metadata.artist) — \(metadata.title) [\(difficulty_name)]" }
    var summary: String {
        let pp = maximum_pp > 0 ? String(format: "%.0f PP максимум", maximum_pp) : "PP недоступен"
        let combo = max_combo > 0 ? "\(max_combo) комбо карты" : "Комбо недоступно"
        return String(format: "%.2f ★ · %.0f BPM · %@ · %@", star_rating, bpm, combo, pp)
    }
}
struct LazerMapEvent: Decodable {
    struct Mod: Decodable { let acronym: String }
    let type: String
    let beatmap: LazerMap
    let mods: [Mod]
    static func decode(_ data: Data) -> LazerMapEvent? {
        guard data.count <= 64 * 1024,
              let event = try? JSONDecoder().decode(Self.self, from: data),
              event.type == "BeatmapStateWebSocketMessage",
              event.beatmap.metadata.title.count <= 1024,
              event.beatmap.metadata.artist.count <= 1024,
              event.beatmap.difficulty_name.count <= 1024,
              (0...1000000).contains(event.beatmap.max_combo),
              event.mods.count <= 64,
              event.mods.allSatisfy({ !$0.acronym.isEmpty && $0.acronym.count <= 8 }),
              [event.beatmap.star_rating, event.beatmap.maximum_pp, event.beatmap.bpm].allSatisfy({ $0.isFinite && $0 >= 0 && $0 <= 1000000 }) else { return nil }
        return event
    }
}

// All mutable state is accessed on the main queue. Network callbacks never
// block AppKit. A PID change/disconnect clears data before reconnection.
final class LazerSocket {
    private(set) var event: LazerMapEvent?
    private var socket: URLSessionWebSocketTask?
    private var pid: Int32 = 0
    private var retryAt = Date.distantPast
    private var pingAt = Date.distantPast
    private var pingPending: Date?
    private var connected = false
    private let session = URLSession(configuration: .ephemeral)
    private let url: URL
    var diagnostic: String {
        connected ? (event == nil ? "API подключён · выберите карту" : "API lazer") : "API недоступен · используется журнал"
    }
    init(port: UInt16 = 49727) { url = URL(string: "ws://localhost:\(port)/")! }
    func tick(pid: Int32) {
        if self.pid != pid { stop(); self.pid = pid }
        guard pid != 0 else { return }
        if let pending = pingPending, Date().timeIntervalSince(pending) > 3 { failed(); return }
        if let task = socket {
            if pingPending == nil && Date().timeIntervalSince(pingAt) >= 1 {
                pingPending = Date(); pingAt = Date()
                task.sendPing { [weak self, weak task] error in
                    DispatchQueue.main.async {
                        guard let self, let task, self.socket === task else { return }
                        self.pingPending = nil
                        if error != nil { self.failed() } else { self.connected = true }
                    }
                }
            }
            return
        }
        guard Date() >= retryAt else { return }
        let task = session.webSocketTask(with: url)
        task.maximumMessageSize = 64 * 1024
        socket = task; task.resume(); receive(task)
    }
    private func receive(_ task: URLSessionWebSocketTask) {
        task.receive { [weak self, weak task] result in
            DispatchQueue.main.async {
                guard let self, let task, self.socket === task else { return }
                switch result {
                case .failure: self.failed()
                case .success(let message):
                    self.connected = true
                    if case .string(let text) = message, let data = text.data(using: .utf8),
                       let envelope = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                       envelope["type"] as? String == "BeatmapStateWebSocketMessage" {
                        // An invalid replacement must not leave the previous map on screen.
                        self.event = LazerMapEvent.decode(data)
                    }
                    self.receive(task)
                }
            }
        }
    }
    private func failed() { stop(); retryAt = Date().addingTimeInterval(3) }
    func stop() {
        socket?.cancel(with: .goingAway, reason: nil); socket = nil
        event = nil; connected = false; pingPending = nil; pingAt = .distantPast
    }
}

func testLazerMessages() {
    let valid = #"{"type":"BeatmapStateWebSocketMessage","beatmap":{"metadata":{"artist":"Artist","title":"Song"},"difficulty_name":"Hard","star_rating":5.25,"maximum_pp":210,"max_combo":1234,"bpm":180},"mods":[{"acronym":"HD"}]}"#
    let event = LazerMapEvent.decode(Data(valid.utf8))!
    precondition(event.beatmap.max_combo == 1234 && event.mods[0].acronym == "HD")
    precondition(event.beatmap.summary.contains("максимум"))
    for invalid in [valid.replacingOccurrences(of: "1234", with: "-1"), valid.replacingOccurrences(of: "5.25", with: "-1"), valid.replacingOccurrences(of: "BeatmapStateWebSocketMessage", with: "Gameplay"), "{}", String(repeating: "x", count: 65537)] {
        precondition(LazerMapEvent.decode(Data(invalid.utf8)) == nil)
    }
    let noPP = LazerMapEvent.decode(Data(valid.replacingOccurrences(of: "210", with: "0").utf8))!
    precondition(noPP.beatmap.summary.contains("недоступен"))
    print("Lazer API message checks passed")
}
