import 'dart:math';

import 'package:fake_async/fake_async.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:passingover/src/location.dart';
import 'package:passingover/src/rust/api/sky.dart';
import 'package:passingover/src/settings.dart';
import 'package:passingover/src/sky_controller.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'support/fakes.dart';

void main() {
  late FakeSkyApi api;
  late FakeLocation location;
  late Settings settings;

  setUp(() async {
    SharedPreferences.setMockInitialValues({});
    settings = Settings(await SharedPreferences.getInstance());
    api = FakeSkyApi();
    location = FakeLocation();
  });

  SkyController controller(FakeAsync async) => SkyController(
    api: api,
    settings: settings,
    location: location,
    random: Random(1),
    clock: () => DateTime(2026, 10, 26).add(async.elapsed),
  );

  test('polls on start, then every 15 ± 2 s, and saves the usage', () {
    fakeAsync((async) {
      final c = controller(async)..start();
      async.flushMicrotasks();
      expect(api.polls, hasLength(1));
      expect(api.polls.single, (59.65, 17.92, Settings.defaultRadiusKm));
      expect(c.view?.flights.single.title, 'SK 945');
      expect(settings.usage?.adsbLol, 1);

      async.elapse(const Duration(seconds: 12));
      expect(api.polls, hasLength(1), reason: 'never before 13 s');
      async.elapse(const Duration(seconds: 5));
      expect(api.polls, hasLength(2), reason: 'by 17 s');
      c.dispose();
    });
  });

  test('pauses while hidden and polls again when shown', () {
    fakeAsync((async) {
      final c = controller(async)..start();
      async.flushMicrotasks();
      c.pause();
      async.elapse(const Duration(minutes: 5));
      expect(api.polls, hasLength(1));
      c.resume();
      async.flushMicrotasks();
      expect(api.polls, hasLength(2));
      c.dispose();
    });
  });

  test('waits as long as the core asks when every source is resting', () {
    fakeAsync((async) {
      api.answers.add(
        const PollResult(error: 'No source', retryAfterSecs: 120),
      );
      final c = controller(async)..start();
      async.flushMicrotasks();
      expect(c.error, 'No source');
      async.elapse(const Duration(seconds: 119));
      expect(api.polls, hasLength(1));
      async.elapse(const Duration(seconds: 2));
      expect(api.polls, hasLength(2));
      c.dispose();
    });
  });

  test('typed-in coordinates replace the device location', () {
    fakeAsync((async) {
      final c = controller(async)..start();
      async.flushMicrotasks();
      c.useManualLocation(const LatLon(55.68, 12.57));
      async.flushMicrotasks();
      expect(api.polls.last.$1, 55.68);
      expect(settings.locationMode, LocationMode.manual);
      expect(location.calls, 1, reason: 'the device is not asked again');
      c.dispose();
    });
  });

  test('re-reads the device location when it is more than 10 minutes old', () {
    fakeAsync((async) {
      final c = controller(async)..start();
      async.flushMicrotasks();
      async.elapse(const Duration(minutes: 9));
      expect(location.calls, 1);
      async.elapse(const Duration(minutes: 2));
      expect(location.calls, 2);
      c.dispose();
    });
  });

  test('without a location it waits for the user', () {
    fakeAsync((async) {
      location.result = const LocationResult.problem('No permission');
      final c = controller(async)..start();
      async.flushMicrotasks();
      expect(c.phase, SkyPhase.needsLocation);
      expect(api.polls, isEmpty);
      c.dispose();
    });
  });
}
