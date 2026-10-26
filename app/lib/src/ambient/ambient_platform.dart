import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:wakelock_plus/wakelock_plus.dart';
import 'package:window_manager/window_manager.dart';

/// What ambient mode asks of the device: stay awake, fill the screen.
abstract class AmbientPlatform {
  Future<void> enter();
  Future<void> exit();
}

class DeviceAmbientPlatform implements AmbientPlatform {
  const DeviceAmbientPlatform();

  static bool get isDesktop =>
      !kIsWeb && (Platform.isMacOS || Platform.isWindows || Platform.isLinux);

  @override
  Future<void> enter() async {
    await _quietly(WakelockPlus.enable);
    if (isDesktop) {
      // macOS ignores full screen for a window that isn't in front.
      await _quietly(windowManager.focus);
      await _quietly(() => windowManager.setFullScreen(true));
    } else {
      await _quietly(
        () => SystemChrome.setEnabledSystemUIMode(SystemUiMode.immersiveSticky),
      );
    }
  }

  @override
  Future<void> exit() async {
    await _quietly(WakelockPlus.disable);
    if (isDesktop) {
      await _quietly(() => windowManager.setFullScreen(false));
    } else {
      await _quietly(
        () => SystemChrome.setEnabledSystemUIMode(SystemUiMode.edgeToEdge),
      );
    }
  }

  /// Ambient mode still works if a platform can't do one of these.
  static Future<void> _quietly(Future<void> Function() action) async {
    try {
      await action();
    } catch (_) {}
  }
}

class NoAmbientPlatform implements AmbientPlatform {
  const NoAmbientPlatform();

  @override
  Future<void> enter() async {}

  @override
  Future<void> exit() async {}
}
