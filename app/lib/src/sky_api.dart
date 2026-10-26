import 'package:flutter/services.dart';

import 'rust/api/sky.dart';

/// What the app needs from the Rust core. Tests use a fake.
abstract class SkyApi {
  /// Load the bundled standing data. Returns false if the app was built
  /// without it (routes are then looked up online only, D29).
  Future<bool> loadBundledData();

  /// What is overhead. `lat`/`lon` never leave the device unrounded.
  Future<PollResult> poll(double lat, double lon, double radiusKm);

  Usage get usage;

  PollTiming get timing;
}

class RustSkyApi implements SkyApi {
  RustSkyApi(this._handle);

  final SkyHandle _handle;

  @override
  Future<bool> loadBundledData() async {
    Future<Uint8List> asset(String name) async {
      final data = await rootBundle.load('assets/data/$name');
      return data.buffer.asUint8List(data.offsetInBytes, data.lengthInBytes);
    }

    try {
      await _handle.loadData(
        routesGz: await asset('routes.tsv.gz'),
        airportsGz: await asset('airports.tsv.gz'),
        airlinesGz: await asset('airlines.tsv.gz'),
      );
      return true;
    } catch (_) {
      return false;
    }
  }

  @override
  Future<PollResult> poll(double lat, double lon, double radiusKm) =>
      _handle.poll(lat: lat, lon: lon, radiusKm: radiusKm);

  @override
  Usage get usage => _handle.usage();

  @override
  PollTiming get timing => pollTiming();
}
