import 'dart:async';
import 'dart:math';

import 'package:flutter/foundation.dart';

import 'location.dart';
import 'rust/api/sky.dart';
import 'settings.dart';
import 'sky_api.dart';

enum SkyPhase {
  /// Loading data and finding the location.
  starting,

  /// No location yet: the user has to allow it or type one in.
  needsLocation,

  /// Polling.
  watching,
}

/// Keeps the "Now" view up to date: polls every ~15 s while visible, pauses
/// while hidden, and saves the daily request count (D15).
class SkyController extends ChangeNotifier {
  SkyController({
    required this._api,
    required this._settings,
    required this._location,
    Random? random,
    DateTime Function()? clock,
  }) : _random = random ?? Random(),
       _clock = clock ?? DateTime.now;

  /// How long a device location is trusted before asking again.
  static const locationMaxAge = Duration(minutes: 10);

  final SkyApi _api;
  final Settings _settings;
  final LocationProvider _location;
  final Random _random;
  final DateTime Function() _clock;

  SkyPhase phase = SkyPhase.starting;
  NowView? view;

  /// Why the last poll produced no view, if it didn't.
  String? error;

  /// Why there is no device location, if there isn't.
  String? locationProblem;
  LatLon? here;
  DateTime? lastUpdate;
  bool hasBundledData = false;

  Timer? _timer;
  bool _paused = false;
  bool _polling = false;
  bool _pollAgain = false;
  DateTime? _locatedAt;

  LocationMode get locationMode => _settings.locationMode;
  LatLon? get manualLocation => _settings.manualLocation;

  Future<void> start() async {
    hasBundledData = await _api.loadBundledData();
    await _locate();
    await _poll();
  }

  /// Stop polling while the app is hidden.
  void pause() {
    _paused = true;
    _timer?.cancel();
  }

  /// Poll again when the app is visible again.
  Future<void> resume() async {
    if (!_paused) return;
    _paused = false;
    await _tick();
  }

  Future<void> useDeviceLocation() async {
    await _settings.setLocationMode(LocationMode.device);
    _locatedAt = null;
    await _locate();
    await _poll();
  }

  Future<void> useManualLocation(LatLon place) async {
    await _settings.setManualLocation(place);
    await _settings.setLocationMode(LocationMode.manual);
    await _locate();
    await _poll();
  }

  Future<void> _tick() async {
    final stale =
        _locatedAt == null || _clock().difference(_locatedAt!) > locationMaxAge;
    if (_settings.locationMode == LocationMode.device && stale) {
      await _locate();
    }
    await _poll();
  }

  Future<void> _locate() async {
    final manual = _settings.manualLocation;
    if (_settings.locationMode == LocationMode.manual && manual != null) {
      here = manual;
      locationProblem = null;
      return;
    }
    final result = await _location.locate();
    if (result.position != null) {
      here = result.position;
      _locatedAt = _clock();
    }
    locationProblem = result.problem;
  }

  Future<void> _poll() async {
    if (_polling) {
      _pollAgain = true;
      return;
    }
    _timer?.cancel();
    if (_paused) return;
    final place = here;
    if (place == null) {
      phase = SkyPhase.needsLocation;
      notifyListeners();
      return;
    }
    phase = SkyPhase.watching;
    _polling = true;
    PollResult result;
    try {
      result = await _api.poll(place.lat, place.lon, _settings.radiusKm);
    } catch (e) {
      result = PollResult(error: e.toString());
    } finally {
      _polling = false;
    }
    await _settings.setUsage(_api.usage);
    if (result.view != null) {
      view = result.view;
      error = null;
      lastUpdate = _clock();
    } else {
      error = result.error;
    }
    notifyListeners();
    if (_pollAgain) {
      _pollAgain = false;
      return _poll();
    }
    _schedule(result.retryAfterSecs);
  }

  void _schedule(int? retryAfterSecs) {
    if (_paused) return;
    final t = _api.timing;
    var ms = t.intervalMs + _random.nextInt(2 * t.jitterMs + 1) - t.jitterMs;
    if (retryAfterSecs != null) ms = max(ms, retryAfterSecs * 1000);
    ms = max(ms, t.minIntervalMs);
    _timer = Timer(Duration(milliseconds: ms), () => unawaited(_tick()));
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }
}
