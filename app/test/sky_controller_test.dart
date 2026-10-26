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
      expect(api.polls.single, (
        lat: 59.65,
        lon: 17.92,
        radiusKm: Settings.defaultRadiusKm,
        imperial: false,
      ));
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

  test('typed-in coordinates replace the device location, politely', () {
    fakeAsync((async) {
      final c = controller(async)..start();
      async.flushMicrotasks();
      c.useManualLocation(const LatLon(55.68, 12.57));
      async.flushMicrotasks();
      expect(api.polls, hasLength(1), reason: 'not within 10 s of a poll');
      async.elapse(const Duration(seconds: 10));
      expect(api.polls.last.lat, 55.68);
      expect(settings.locationMode, LocationMode.manual);
      expect(location.calls, 1, reason: 'the device is not asked again');
      c.dispose();
    });
  });

  test('switching to °F polls again with imperial units', () {
    fakeAsync((async) {
      final c = controller(async)..start();
      async.flushMicrotasks();
      async.elapse(const Duration(seconds: 11));
      c.setImperial(true);
      async.flushMicrotasks();
      expect(api.polls, hasLength(2));
      expect(api.polls.last.imperial, isTrue);
      expect(settings.imperial, isTrue);
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

  test('ambient mode polls every minute, then back to every 15 s', () {
    fakeAsync((async) {
      final c = controller(async)..start();
      async.flushMicrotasks();
      c.setAmbient(true);
      async.elapse(const Duration(seconds: 50));
      expect(api.polls, hasLength(1));
      async.elapse(const Duration(seconds: 13));
      expect(api.polls, hasLength(2));
      c.setAmbient(false);
      async.flushMicrotasks();
      async.elapse(const Duration(seconds: 17));
      expect(api.polls.length, greaterThanOrEqualTo(3));
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
