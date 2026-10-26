import 'package:flutter/foundation.dart';
import 'package:geolocator/geolocator.dart';

import 'settings.dart';

/// A position, or a calm explanation of why there isn't one.
class LocationResult {
  const LocationResult.found(LatLon this.position) : problem = null;
  const LocationResult.problem(String this.problem) : position = null;

  final LatLon? position;
  final String? problem;
}

abstract class LocationProvider {
  Future<LocationResult> locate();
}

/// The device's location, asked for at low accuracy: Passing Over only needs
/// to know roughly where you are, and rounds it to ~1 km before sending.
class DeviceLocation implements LocationProvider {
  @override
  Future<LocationResult> locate() async {
    try {
      if (!await Geolocator.isLocationServiceEnabled()) {
        return const LocationResult.problem(
          'Location services are turned off.',
        );
      }
      var permission = await Geolocator.checkPermission();
      if (permission == LocationPermission.denied) {
        permission = await Geolocator.requestPermission();
      }
      if (permission == LocationPermission.denied ||
          permission == LocationPermission.deniedForever) {
        return const LocationResult.problem(
          'Passing Over doesn\'t have permission to use your location.',
        );
      }
      const timeLimit = Duration(seconds: 30);
      final position = await Geolocator.getCurrentPosition(
        // On Android, use the platform's LocationManager rather than Google
        // Play services: it needs no Google account or "Location Accuracy"
        // prompt, and coarse location is all we ask for (D31).
        locationSettings: defaultTargetPlatform == TargetPlatform.android
            ? AndroidSettings(
                accuracy: LocationAccuracy.low,
                timeLimit: timeLimit,
                forceLocationManager: true,
              )
            : const LocationSettings(
                accuracy: LocationAccuracy.low,
                timeLimit: timeLimit,
              ),
      );
      return LocationResult.found(
        LatLon(position.latitude, position.longitude),
      );
    } catch (_) {
      return const LocationResult.problem('Couldn\'t find your location.');
    }
  }
}
