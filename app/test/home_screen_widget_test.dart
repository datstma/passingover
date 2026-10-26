import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:passingover/src/home_screen_widget.dart';
import 'package:passingover/src/settings.dart';
import 'package:passingover/src/sky_controller.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'support/fakes.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final evening = DateTime(2026, 10, 26, 21, 30);

  test('shows the nearest flight with a destination, and when', () {
    expect(widgetContent(viewOf([unknown, sk945]), evening), {
      'title': 'SK 945',
      'destination': '→ Chicago',
      'details': '15:12 there, 9°C and overcast · landing in about 7 h 40 min',
      'updated': 'As of 21:30',
    });
  });

  test('a flight landing nearby skips the time and weather there', () {
    expect(
      widgetContent(viewOf([aa43]), evening)['details'],
      'Landing nearby in about 5 min',
    );
  });

  test('a quiet sky says so', () {
    expect(widgetContent(viewOf([]), evening)['destination'], 'Quiet sky');
    expect(
      widgetContent(viewOf([unknown]), evening)['destination'],
      'Destination unknown',
    );
  });

  test('the service hands each new view to the native widget', () async {
    final calls = <MethodCall>[];
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(const MethodChannel('home_widget'), (
          call,
        ) async {
          calls.add(call);
          return true;
        });
    SharedPreferences.setMockInitialValues({});
    final controller = SkyController(
      api: FakeSkyApi(),
      settings: Settings(await SharedPreferences.getInstance()),
      location: FakeLocation(),
    );
    HomeScreenWidget(controller).start();
    await controller.start();
    await Future<void>.delayed(Duration.zero);
    final saved = {
      for (final c in calls.where((c) => c.method == 'saveWidgetData'))
        (c.arguments as Map)['id']: (c.arguments as Map)['data'],
    };
    expect(saved['destination'], '→ Chicago');
    expect(calls.last.method, 'updateWidget');
    expect((calls.last.arguments as Map)['android'], 'PassingOverWidget');

    // The same view again changes nothing.
    final count = calls.length;
    controller.notifyListeners();
    await Future<void>.delayed(Duration.zero);
    expect(calls, hasLength(count));
    controller.dispose();
  });
}
