import 'package:flutter/services.dart';
import 'package:path_provider/path_provider.dart';

import 'rust/api/sky.dart';

/// What the app needs from the Rust core. Tests use a fake.
abstract class SkyApi {
  /// Load the bundled standing data and regions. Returns false if the app
  /// was built without them (routes are then looked up online only, D29).
  Future<bool> loadBundledData();

  /// What is overhead. `lat`/`lon` never leave the device unrounded.
  Future<PollResult> poll(
    double lat,
    double lon,
    double radiusKm, {
    required bool imperial,
  });

  Usage get usage;

  PollTiming get timing;

  /// Start keeping the daydream log in the app's private folder (D38).
  Future<void> openLog();

  void closeLog();

  Future<void> clearLog();

  /// Sentences about the log since `since`, labelled `period`.
  Future<List<String>> logSummary(
    DateTime since,
    String period, {
    double? lat,
    double? lon,
    required bool imperial,
  });
}

class RustSkyApi implements SkyApi {
  RustSkyApi(this._handle);

  final SkyHandle _handle;

  static Future<Uint8List> _asset(String name) async {
    final data = await rootBundle.load('assets/data/$name');
    return data.buffer.asUint8List(data.offsetInBytes, data.lengthInBytes);
  }

  @override
  Future<bool> loadBundledData() async {
    try {
      await _handle.loadData(
        routesGz: await _asset('routes.tsv.gz'),
        airportsGz: await _asset('airports.tsv.gz'),
        airlinesGz: await _asset('airlines.tsv.gz'),
      );
    } catch (_) {
      return false;
    }
    try {
      await _handle.loadRegions(regionsGz: await _asset('regions.tsv.gz'));
    } catch (_) {
      // Without regions the journey just doesn't say what lies ahead.
    }
    return true;
  }

  @override
  Future<PollResult> poll(
    double lat,
    double lon,
    double radiusKm, {
    required bool imperial,
  }) =>
      _handle.poll(lat: lat, lon: lon, radiusKm: radiusKm, imperial: imperial);

  @override
  Usage get usage => _handle.usage();

  @override
  PollTiming get timing => pollTiming();

  @override
  Future<void> openLog() async {
    final dir = await getApplicationSupportDirectory();
    await dir.create(recursive: true);
    await _handle.openLog(path: '${dir.path}/daydream-log.tsv');
  }

  @override
  void closeLog() => _handle.closeLog();

  @override
  Future<void> clearLog() => _handle.clearLog();

  @override
  Future<List<String>> logSummary(
    DateTime since,
    String period, {
    double? lat,
    double? lon,
    required bool imperial,
  }) => _handle.logSummary(
    sinceUnixMs: since.millisecondsSinceEpoch,
    period: period,
    lat: lat,
    lon: lon,
    imperial: imperial,
  );
}
