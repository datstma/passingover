import 'dart:async';
import 'dart:math';

import 'package:flutter/foundation.dart';

import 'location.dart';
import 'rust/api/sky.dart';
import 'settings.dart';
import 'sky_api.dart';

/// A stretch of the daydream log.
enum LogPeriod {
  week('This week'),
  month('This month'),
  all('So far');

  const LogPeriod(this.label);

  final String label;

  /// Local start of the period, as of `now`.
  DateTime since(DateTime now) => switch (this) {
    LogPeriod.week => DateTime(
      now.year,
      now.month,
      now.day - (now.weekday - 1),
    ),
    LogPeriod.month => DateTime(now.year, now.month),
    LogPeriod.all => DateTime.fromMillisecondsSinceEpoch(0),
  };
}

enum SkyPhase {
  /// Loading data and finding the location.
  starting,

  /// No location yet: the user has to allow it or type one in.
  needsLocation,

  /// Polling.
  watching,
}

/// Keeps the "Now" view up to date: polls every ~15 s while visible, pauses
/// while hidden, never polls more often than the core allows, and saves the
/// daily request count (D15).
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
  bool _ambient = false;

  /// Hidden, but something (the menu bar item) still shows flights: keep
  /// polling at the ambient pace instead of stopping (D39).
  bool _keepPollingWhenHidden = false;
  bool _hiddenButShown = false;
  bool _polling = false;
  bool _pollAgain = false;
  DateTime? _locatedAt;
  DateTime? _lastPollAt;

  Settings get settings => _settings;
  LocationMode get locationMode => _settings.locationMode;
  LatLon? get manualLocation => _settings.manualLocation;
  bool get imperial => _settings.imperial;

  Future<void> start() async {
    hasBundledData = await _api.loadBundledData();
    if (_settings.keepLog) {
      try {
        await _api.openLog();
      } catch (_) {
        // Without a log the app still works.
      }
    }
    await _locate();
    await _poll();
  }

  /// Stop polling while the app is hidden, or slow down to the ambient
  /// pace if the menu bar item still shows flights.
  void pause() {
    _timer?.cancel();
    if (_keepPollingWhenHidden) {
      _hiddenButShown = true;
      if (!_polling) _schedule(null);
    } else {
      _paused = true;
    }
  }

  /// Poll again when the app is visible again.
  Future<void> resume() async {
    if (_hiddenButShown) {
      _hiddenButShown = false;
      _pollSoon();
      return;
    }
    if (!_paused) return;
    _paused = false;
    await _tick();
  }

  void setKeepPollingWhenHidden(bool keep) => _keepPollingWhenHidden = keep;

  bool get showInMenuBar => _settings.showInMenuBar;

  Future<void> setShowInMenuBar(bool show) async {
    await _settings.setShowInMenuBar(show);
    notifyListeners();
  }

  Future<void> useDeviceLocation() async {
    await _settings.setLocationMode(LocationMode.device);
    _locatedAt = null;
    await _locate();
    _pollSoon();
  }

  Future<void> useManualLocation(LatLon place) async {
    await _settings.setManualLocation(place);
    await _settings.setLocationMode(LocationMode.manual);
    await _locate();
    _pollSoon();
  }

  /// Ambient mode polls less often (D15: every 60 s).
  void setAmbient(bool ambient) {
    if (_ambient == ambient) return;
    _ambient = ambient;
    if (!ambient) {
      _pollSoon();
    } else if (!_polling && !_paused && here != null) {
      _schedule(null);
    }
  }

  bool get keepLog => _settings.keepLog;

  Future<void> setKeepLog(bool keep) async {
    await _settings.setKeepLog(keep);
    if (keep) {
      await _api.openLog();
    } else {
      _api.closeLog();
    }
    notifyListeners();
  }

  Future<void> clearLog() async {
    await _api.clearLog();
    notifyListeners();
  }

  Future<List<String>> logSummary(LogPeriod period) => _api.logSummary(
    period.since(_clock()),
    period.label,
    lat: here?.lat,
    lon: here?.lon,
    imperial: _settings.imperial,
  );

  Future<void> setImperial(bool imperial) async {
    await _settings.setImperial(imperial);
    notifyListeners();
    _pollSoon();
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

  /// Poll as soon as the minimum interval since the last poll allows.
  void _pollSoon() {
    if (_paused) return;
    final last = _lastPollAt;
    final min = Duration(milliseconds: _api.timing.minIntervalMs);
    final wait = last == null ? Duration.zero : min - _clock().difference(last);
    _timer?.cancel();
    if (wait <= Duration.zero) {
      unawaited(_poll());
    } else {
      _timer = Timer(wait, () => unawaited(_poll()));
    }
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
    _lastPollAt = _clock();
    PollResult result;
    try {
      result = await _api.poll(
        place.lat,
        place.lon,
        _settings.radiusKm,
        imperial: _settings.imperial,
      );
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
      _pollSoon();
      return;
    }
    _schedule(result.retryAfterSecs);
  }

  void _schedule(int? retryAfterSecs) {
    if (_paused) return;
    final t = _api.timing;
    final slow = _ambient || _hiddenButShown;
    final base = slow ? t.ambientIntervalMs : t.intervalMs;
    var ms = base + _random.nextInt(2 * t.jitterMs + 1) - t.jitterMs;
    if (retryAfterSecs != null) ms = max(ms, retryAfterSecs * 1000);
    ms = max(ms, t.minIntervalMs);
    _timer?.cancel();
    _timer = Timer(Duration(milliseconds: ms), () => unawaited(_tick()));
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }
}
