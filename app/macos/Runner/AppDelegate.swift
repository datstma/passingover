import Cocoa
import FlutterMacOS

@main
class AppDelegate: FlutterAppDelegate {
  /// With the menu bar item shown, Passing Over keeps running when its
  /// window is closed (D39).
  override func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
    return !MenuBar.shared.isActive
  }

  /// Clicking the Dock icon brings a hidden window back.
  override func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool)
    -> Bool
  {
    if !flag { MenuBar.shared.showWindow() }
    return true
  }

  override func applicationSupportsSecureRestorableState(_ app: NSApplication) -> Bool {
    return true
  }
}
