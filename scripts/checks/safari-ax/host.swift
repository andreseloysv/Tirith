// A throwaway WebKit window for timing Accessibility scripts: it shows one local page and never takes the focus.
// Not Safari and not Chrome: a window of its own, behind the others, closed when its process ends.
import AppKit
import WebKit

let page = URL(fileURLWithPath: CommandLine.arguments[1])
let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let frame = NSRect(x: 40, y: 80, width: 900, height: 700)
let window = NSWindow(contentRect: frame, styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false)
window.title = "NeoBot AX test"
let web = WKWebView(frame: frame)
window.contentView = web
web.loadFileURL(page, allowingReadAccessTo: page.deletingLastPathComponent())
window.orderBack(nil)
print("pid \(ProcessInfo.processInfo.processIdentifier)")
fflush(stdout)
// Ends by itself: a test that forgets it leaves nothing behind.
DispatchQueue.main.asyncAfter(deadline: .now() + 900) { exit(0) }
app.run()
