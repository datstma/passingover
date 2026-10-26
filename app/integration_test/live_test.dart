import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:passingover/main.dart';
import 'package:passingover/src/location.dart';
import 'package:passingover/src/rust/api/sky.dart';
import 'package:passingover/src/rust/frb_generated.dart';
import 'package:passingover/src/settings.dart';
import 'package:passingover/src/sky_api.dart';
import 'package:passingover/src/sky_controller.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// Opt-in: talks to the real aircraft sources, so it never runs by default:
///
/// ```sh
/// flutter test integration_test/live_test.dart -d macos \
///   --dart-define=PASSINGOVER_LIVE=true
/// ```
///
/// It prints what it saw (lines starting `LIVE`).
const live = bool.fromEnvironment('PASSINGOVER_LIVE');

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('the Now screen fills from the real sources', (tester) async {
    if (!live) {
      markTestSkipped('set --dart-define=PASSINGOVER_LIVE=true');
      return;
    }
    await RustLib.init();
    // A public place: central Copenhagen.
    SharedPreferences.setMockInitialValues({
      'location.mode': 'manual',
      'location.lat': 55.676,
      'location.lon': 12.568,
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
    debugPrint(
      'LIVE flights: ${controller.view!.flights.map((f) => f.line).toList()}',
    );
    debugPrint('LIVE bundled data: ${controller.hasBundledData}');

    controller.dispose();
  });
}
