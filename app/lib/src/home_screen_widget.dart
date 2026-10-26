import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:home_widget/home_widget.dart';

import 'rust/api/sky.dart';
import 'sky_controller.dart';

/// What the Android home-screen widget shows (D40): the nearest flight the
/// app saw last, and when.
Map<String, String> widgetContent(NowView view, DateTime updated) {
  String two(int n) => n.toString().padLeft(2, '0');
  final asOf = 'As of ${two(updated.hour)}:${two(updated.minute)}';
  final flights = view.flights;
  if (flights.isEmpty) {
    return {
      'title': 'Passing Over',
      'destination': 'Quiet sky',
      'details': 'No planes overhead just now.',
      'updated': asOf,
    };
  }
  final f = flights.firstWhere(
    (f) => f.destination != null,
    orElse: () => flights.first,
  );
  final there = [
    if (f.localTime != null) '${f.localTime} there',
    ?f.weather,
  ].join(', ');
  final details = f.landsNearby
      ? (f.landingIn == null
            ? 'Landing nearby'
            : 'Landing nearby in ${f.landingIn}')
      : [
          if (there.isNotEmpty) there,
          if (f.landingIn != null) 'landing in ${f.landingIn}',
        ].join(' · ');
  return {
    'title': f.title,
    'destination': f.destination == null
        ? 'Destination unknown'
        : '→ ${f.destination}',
    'details': details,
    'updated': asOf,
  };
}

/// Hands the latest view to the native widget
/// (android/…/PassingOverWidget.kt) whenever it changes.
class HomeScreenWidget {
  HomeScreenWidget(this.controller);

  final SkyController controller;
  String? _sent;

  static bool get supported => !kIsWeb && Platform.isAndroid;

  void start() => controller.addListener(_sync);

  void stop() => controller.removeListener(_sync);

  Future<void> _sync() async {
    final view = controller.view;
    final updated = controller.lastUpdate;
    if (view == null || updated == null) return;
    final content = widgetContent(view, updated);
    final encoded = jsonEncode(content);
    if (encoded == _sent) return;
    _sent = encoded;
    try {
      for (final entry in content.entries) {
        await HomeWidget.saveWidgetData<String>(entry.key, entry.value);
      }
      await HomeWidget.updateWidget(androidName: 'PassingOverWidget');
    } catch (_) {
      // The app works the same without the widget.
      _sent = null;
    }
  }
}
