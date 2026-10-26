import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:passingover/src/log_screen.dart';
import 'package:passingover/src/settings.dart';
import 'package:passingover/src/sky_controller.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'support/fakes.dart';

void main() {
  late FakeSkyApi api;
  late SkyController controller;

  Future<void> show(WidgetTester tester, {bool keep = true}) async {
    SharedPreferences.setMockInitialValues({'log.keep': keep});
    api = FakeSkyApi();
    controller = SkyController(
      api: api,
      settings: Settings(await SharedPreferences.getInstance()),
      location: FakeLocation(),
    );
    await controller.start();
    await tester.pumpWidget(
      MaterialApp(home: LogScreen(controller: controller)),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('shows the summary for this month', (tester) async {
    await show(tester);
    expect(api.logOpen, isTrue);
    expect(find.textContaining('61 cities in 24 countries'), findsOneWidget);
    expect(
      find.text('Most often: London (34) and Paris (21).'),
      findsOneWidget,
    );
    await tester.tap(find.text('So far'));
    await tester.pumpAndSettle();
    expect(api.summaryPeriods, ['This month', 'So far']);
    controller.dispose();
  });

  testWidgets('can be turned off and cleared', (tester) async {
    await show(tester);
    await tester.tap(find.byType(Switch));
    await tester.pumpAndSettle();
    expect(api.logOpen, isFalse);
    expect(controller.settings.keepLog, isFalse);
    expect(find.text('The daydream log is off.'), findsOneWidget);

    await tester.tap(find.byType(Switch));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Clear the log'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Clear'));
    await tester.pumpAndSettle();
    expect(find.text('Nothing here yet.'), findsOneWidget);
    controller.dispose();
  });

  test('periods start on Monday and on the 1st, in local time', () {
    final sunday = DateTime(2026, 11, 1, 15, 30);
    expect(LogPeriod.week.since(sunday), DateTime(2026, 10, 26));
    expect(LogPeriod.month.since(sunday), DateTime(2026, 11));
    expect(LogPeriod.all.since(sunday).millisecondsSinceEpoch, 0);
  });

  testWidgets('a log turned off at start is not opened', (tester) async {
    await show(tester, keep: false);
    expect(api.logOpen, isFalse);
    controller.dispose();
  });
}
