import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'ambient/ambient_screen.dart';
import 'rust/api/sky.dart';
import 'sky_controller.dart';

/// What the macOS menu bar item shows (D39): the nearest destination as its
/// title, the flights' sentences in its menu.
Map<String, Object> menuBarContent(NowView? view, DateTime? updated) {
  final flights = view?.flights ?? const <FlightView>[];
  final nearest = flights.where((f) => f.destination != null).firstOrNull;
  final lines = flights.isEmpty && view != null
      ? ['The sky above you is quiet right now.']
      : [for (final f in flights.take(8)) f.line];
  String two(int n) => n.toString().padLeft(2, '0');
  final source = view == null
      ? null
      : switch (view.source) {
          Source.adsbLol => 'adsb.lol',
          Source.adsbFi => 'adsb.fi',
        };
  return {
    'title': nearest?.destination ?? '',
    'lines': lines,
    'footer': updated == null
        ? ''
        : 'Updated ${two(updated.hour)}:${two(updated.minute)} · $source',
  };
}

/// Keeps the native menu bar item (macos/Runner/MainFlutterWindow.swift)
/// in step with the controller.
class MacMenuBar {
  MacMenuBar({
    required this.controller,
    required this.navigatorKey,
    MethodChannel? channel,
  }) : _channel = channel ?? const MethodChannel('passingover/menu_bar');

  final SkyController controller;
  final GlobalKey<NavigatorState> navigatorKey;
  final MethodChannel _channel;
  String? _sent;
  bool _shown = false;

  static bool get supported => !kIsWeb && Platform.isMacOS;

  void start() {
    _channel.setMethodCallHandler((call) async {
      if (call.method == 'openAmbient') {
        await navigatorKey.currentState?.push(
          MaterialPageRoute<void>(
            builder: (_) => AmbientScreen(controller: controller),
          ),
        );
      }
    });
    controller.addListener(_sync);
    _sync();
  }

  void stop() {
    controller.removeListener(_sync);
    _channel.setMethodCallHandler(null);
  }

  void _sync() {
    final show = controller.showInMenuBar;
    controller.setKeepPollingWhenHidden(show);
    if (!show) {
      if (_shown) {
        _shown = false;
        _sent = null;
        _channel.invokeMethod<void>('remove');
      }
      return;
    }
    final content = menuBarContent(controller.view, controller.lastUpdate);
    final encoded = jsonEncode(content);
    if (encoded == _sent) return;
    _sent = encoded;
    _shown = true;
    _channel.invokeMethod<void>('update', content);
  }
}
