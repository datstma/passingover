import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:passingover/main.dart';
import 'package:passingover/src/ambient/ambient_platform.dart';
import 'package:passingover/src/ambient/ambient_screen.dart';
import 'package:passingover/src/location.dart';
import 'package:passingover/src/rust/api/hello.dart';
import 'package:passingover/src/rust/api/sky.dart';
import 'package:passingover/src/rust/frb_generated.dart';
import 'package:passingover/src/settings.dart';
import 'package:passingover/src/sky_api.dart';
import 'package:passingover/src/sky_controller.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:window_manager/window_manager.dart';

/// The last test talks to the real aircraft sources, so it only runs with
/// `--dart-define=PASSINGOVER_LIVE=true`, e.g.
///
/// ```sh
/// flutter test integration_test -d macos --dart-define=PASSINGOVER_LIVE=true
/// ```
///
/// All tests live in this one file: desktop runs start the app once per file,
/// and a second start fails on Linux and Windows.
const live = bool.fromEnvironment('PASSINGOVER_LIVE');

/// Where the live test looks (a public place): central Copenhagen unless
/// `--dart-define=PASSINGOVER_LIVE_AT=lat,lon` says otherwise.
const liveAt = String.fromEnvironment(
  'PASSINGOVER_LIVE_AT',
  defaultValue: '55.676,12.568',
);

/// A minimal adsb.lol response: one airliner, one tower.
final _snapshot = utf8.encode(
  jsonEncode({
    'ac': [
      {
        'hex': '4ac9e2',
        'flight': 'SAS2802 ',
        't': 'A20N',
        'category': 'A3',
        'lat': 55.65,
        'lon': 12.70,
        'alt_baro': 6900,
        'gs': 251.3,
        'track': 64.6,
        'seen_pos': 0.2,
      },
      {
        'hex': '45fe4b',
        't': 'TWR',
        'lat': 55.62,
        'lon': 12.65,
        'alt_baro': 'ground',
      },
    ],
    'now': 1792966614001,
    'msg': 'No error',
  }),
);

/// Runs on the target platform with the real Rust library: proves the bridge
/// works there. Only the live test touches the network.
void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  test('the Rust core answers', () {
    expect(hello(), startsWith('Hello from passingover-core'));
    expect(pollTiming().intervalMs, 15000);
    expect(dataCredits().map((c) => c.url), contains('https://adsb.fi'));
  });

  test('a recorded snapshot becomes flight views', () async {
    final sky = SkyHandle(primary: Source.adsbLol, seed: 1);
    final view = await sky.viewFromSnapshot(
      source: Source.adsbLol,
      body: _snapshot,
      lat: 55.62,
      lon: 12.66,
      radiusKm: 30,
      imperial: false,
    );
    expect(view.flights, hasLength(1), reason: 'the tower is not flying');
    expect(view.flights.single.title, 'SAS2802');
    expect(view.flights.single.landsNearby, isFalse);
    expect(
      view.flights.single.destination,
      isNull,
      reason: 'no route data loaded',
    );
    expect(view.credits.single.url, 'https://adsb.lol');
  });

  testWidgets('live: the Now screen fills from the real sources', (
    tester,
  ) async {
    final [lat, lon] = liveAt.split(',').map(double.parse).toList();
    SharedPreferences.setMockInitialValues({
      'location.mode': 'manual',
      'location.lat': lat,
      'location.lon': lon,
    });
    final settings = Settings(await SharedPreferences.getInstance());
    final controller = SkyController(
      api: RustSkyApi(SkyHandle(primary: Source.adsbLol, seed: 7)),
      settings: settings,
      location: DeviceLocation(),
    );
    await tester.pumpWidget(PassingOverApp(controller: controller));
    await tester.runAsync(controller.start);
    await tester.pumpAndSettle(const Duration(seconds: 1));

    expect(controller.error, isNull, reason: 'the poll worked');
    expect(controller.view, isNotNull);
    for (final f in controller.view!.flights) {
      debugPrint('LIVE ${f.line} ${f.journey ?? ''}');
    }
    debugPrint('LIVE credits: ${controller.view!.credits.map((c) => c.text)}');
    debugPrint('LIVE bundled data: ${controller.hasBundledData}');

    // Ambient mode really fills the screen on desktops (D36).
    if (DeviceAmbientPlatform.isDesktop) {
      await windowManager.ensureInitialized();
      final navigator = tester.state<NavigatorState>(find.byType(Navigator));
      unawaited(
        navigator.push(
          MaterialPageRoute<void>(
            builder: (_) => AmbientScreen(controller: controller),
          ),
        ),
      );
      await tester.pump();
      // The screen's platform calls need real time, not test frames.
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(seconds: 3)),
      );
      final full = await tester.runAsync(windowManager.isFullScreen);
      debugPrint('LIVE ambient fullscreen: $full');
      expect(full, isTrue);
      navigator.pop();
      await tester.pump();
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(seconds: 3)),
      );
      final back = await tester.runAsync(windowManager.isFullScreen);
      expect(back, isFalse);
    }

    controller.dispose();
  }, skip: !live);
}
