import Cocoa
import FlutterMacOS

class MainFlutterWindow: NSWindow {
  override func awakeFromNib() {
    let flutterViewController = FlutterViewController()
    let windowFrame = self.frame
    self.contentViewController = flutterViewController
    self.setFrame(windowFrame, display: true)

    RegisterGeneratedPlugins(registry: flutterViewController)
    MenuBar.shared.attach(
      messenger: flutterViewController.engine.binaryMessenger, window: self)

    super.awakeFromNib()
  }

  /// While the menu bar item is shown, closing the window hides it, so the
  /// item keeps updating and the window can come back.
  override func close() {
    if MenuBar.shared.isActive {
      orderOut(nil)
    } else {
      super.close()
    }
  }
}

/// The menu bar item (M6, D39): an airplane and the nearest flight's
/// destination, with a menu of the flights overhead. Dart sends what to
/// show over the `passingover/menu_bar` channel.
final class MenuBar: NSObject {
  static let shared = MenuBar()

  private var channel: FlutterMethodChannel?
  private weak var window: NSWindow?
  private var item: NSStatusItem?

  var isActive: Bool { item != nil }

  func attach(messenger: FlutterBinaryMessenger, window: NSWindow) {
    self.window = window
    let channel = FlutterMethodChannel(name: "passingover/menu_bar", binaryMessenger: messenger)
    channel.setMethodCallHandler { [weak self] call, result in
      guard let self else { return }
      switch call.method {
      case "update":
        let args = call.arguments as? [String: Any] ?? [:]
        self.update(
          title: args["title"] as? String ?? "",
          lines: args["lines"] as? [String] ?? [],
          footer: args["footer"] as? String ?? "")
        result(nil)
      case "remove":
        self.remove()
        result(nil)
      default:
        result(FlutterMethodNotImplemented)
      }
    }
    self.channel = channel
  }

  private func update(title: String, lines: [String], footer: String) {
    let item = self.item ?? NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
    self.item = item
    if let button = item.button {
      button.image = NSImage(systemSymbolName: "airplane", accessibilityDescription: "Passing Over")
      button.imagePosition = .imageLeading
      button.title = title.isEmpty ? "" : " \(title)"
    }
    let menu = NSMenu()
    menu.autoenablesItems = false
    for line in lines {
      let entry = NSMenuItem(title: line, action: nil, keyEquivalent: "")
      entry.isEnabled = false
      menu.addItem(entry)
    }
    if !footer.isEmpty {
      if !lines.isEmpty { menu.addItem(.separator()) }
      let entry = NSMenuItem(title: footer, action: nil, keyEquivalent: "")
      entry.isEnabled = false
      menu.addItem(entry)
    }
    menu.addItem(.separator())
    menu.addItem(action("Show Passing Over", #selector(showWindow), "o"))
    menu.addItem(action("Ambient mode", #selector(openAmbient), "a"))
    menu.addItem(.separator())
    menu.addItem(action("Quit Passing Over", #selector(quit), "q"))
    item.menu = menu
  }

  private func action(_ title: String, _ selector: Selector, _ key: String) -> NSMenuItem {
    let entry = NSMenuItem(title: title, action: selector, keyEquivalent: key)
    entry.target = self
    return entry
  }

  private func remove() {
    if let item { NSStatusBar.system.removeStatusItem(item) }
    item = nil
  }

  @objc func showWindow() {
    NSApp.activate(ignoringOtherApps: true)
    window?.makeKeyAndOrderFront(nil)
  }

  @objc private func openAmbient() {
    showWindow()
    channel?.invokeMethod("openAmbient", arguments: nil)
  }

  @objc private func quit() {
    NSApp.terminate(nil)
  }
}
