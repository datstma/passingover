import 'package:flutter/services.dart';

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
}
