import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:passingover/src/ambient/ambient_platform.dart';
import 'package:passingover/src/ambient/ambient_screen.dart';
import 'package:passingover/src/ambient/led_text.dart';
import 'package:passingover/src/rust/api/sky.dart';
import 'package:passingover/src/settings.dart';
import 'package:passingover/src/sky_controller.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'support/fakes.dart';

void main() {
  late SkyController controller;

  Future<void> show(WidgetTester tester, List<FlightView> flights) async {
    SharedPreferences.setMockInitialValues({});
    final api = FakeSkyApi()..answers.add(PollResult(view: viewOf(flights)));
    controller = SkyController(
      api: api,
      settings: Settings(await SharedPreferences.getInstance()),
      location: FakeLocation(),
    );
    await controller.start();
    await tester.pumpWidget(
      MaterialApp(
        home: AmbientScreen(
          controller: controller,
          platform: const NoAmbientPlatform(),
          clock: () => DateTime(2026, 10, 26, 12),
        ),
      ),
    );
    // The star field twinkles forever, so pump instead of settling.
    await tester.pump(const Duration(seconds: 2));
  }

  Future<void> close(WidgetTester tester) async {
    await tester.pumpWidget(const SizedBox());
    controller.dispose();
  }

  testWidgets('shows one flight at a time and moves on', (tester) async {
    await show(tester, [sk945, aa43]);
    expect(find.text('Chicago'), findsOneWidget);
    expect(find.text('15:12 there, 9°C and overcast'), findsOneWidget);
    await tester.pump(ambientCycle);
    await tester.pump(const Duration(seconds: 2));
    expect(find.text('New York'), findsOneWidget);
    expect(find.text('Landing nearby in about 5 min'), findsOneWidget);
    await close(tester);
  });

  testWidgets('a quiet sky says so', (tester) async {
    await show(tester, []);
    expect(find.text('The sky above you is quiet right now.'), findsOneWidget);
    await close(tester);
  });

  testWidgets('tapping shows controls; the LED theme draws dots', (
    tester,
  ) async {
    await show(tester, [sk945]);
    expect(find.byType(LedText), findsNothing);
    await tester.tap(find.byType(AmbientScreen));
    await tester.pump(const Duration(milliseconds: 400));
    await tester.tap(find.text('LED'));
    await tester.pump(const Duration(seconds: 1));
    expect(find.byType(LedText), findsWidgets);
    expect(controller.settings.ambientTheme, AmbientTheme.ledMatrix);
    await close(tester);
  });
}
