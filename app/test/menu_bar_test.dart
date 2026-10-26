import 'package:fake_async/fake_async.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:passingover/src/menu_bar.dart';
import 'package:passingover/src/settings.dart';
import 'package:passingover/src/sky_controller.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'support/fakes.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  const channel = MethodChannel('passingover/menu_bar');
  final calls = <MethodCall>[];

  setUp(() {
    calls.clear();
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          calls.add(call);
          return null;
        });
  });

  test('the title is the nearest destination; the menu, the sentences', () {
    final content = menuBarContent(
      viewOf([unknown, sk945, aa43]),
      DateTime(2026, 10, 26, 3, 1),
    );
    expect(content['title'], 'Chicago');
    expect(content['lines'], [unknown.line, sk945.line, aa43.line]);
    expect(content['footer'], 'Updated 03:01 · adsb.lol');
    expect(menuBarContent(viewOf([]), null)['lines'], [
      'The sky above you is quiet right now.',
    ]);
    expect(menuBarContent(null, null)['title'], '');
  });

  test('updates the item and removes it when turned off', () async {
    SharedPreferences.setMockInitialValues({});
    final controller = SkyController(
      api: FakeSkyApi(),
      settings: Settings(await SharedPreferences.getInstance()),
      location: FakeLocation(),
    );
    final bar = MacMenuBar(
      controller: controller,
      navigatorKey: GlobalKey<NavigatorState>(),
    )..start();
    await controller.start();
    await Future<void>.delayed(Duration.zero);
    expect(calls.map((c) => c.method), contains('update'));
    expect((calls.last.arguments as Map)['title'], 'Chicago');

    await controller.setShowInMenuBar(false);
    expect(calls.last.method, 'remove');
    bar.stop();
    controller.dispose();
  });

  test(
    'keeps polling every minute while hidden if the menu bar shows',
    () async {
      SharedPreferences.setMockInitialValues({});
      final settings = Settings(await SharedPreferences.getInstance());
      fakeAsync((async) {
        final api = FakeSkyApi();
        final controller = SkyController(
          api: api,
          settings: settings,
          location: FakeLocation(),
        )..setKeepPollingWhenHidden(true);
        controller.start();
        async.flushMicrotasks();
        controller.pause();
        async.elapse(const Duration(seconds: 50));
        expect(api.polls, hasLength(1));
        async.elapse(const Duration(seconds: 13));
        expect(api.polls, hasLength(2));
        controller.dispose();
      });
    },
  );
}
