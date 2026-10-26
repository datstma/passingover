import 'package:shared_preferences/shared_preferences.dart';

import 'ambient/dimming.dart';
import 'rust/api/sky.dart';

export 'ambient/dimming.dart' show DimSchedule;

enum AmbientTheme { nightSky, ledMatrix }

/// Where the user's location comes from.
enum LocationMode { device, manual }

class LatLon {
  const LatLon(this.lat, this.lon);

  final double lat;
  final double lon;

  /// Whether this is a valid WGS 84 position.
  bool get isValid =>
      lat.isFinite &&
      lon.isFinite &&
      lat >= -90 &&
      lat <= 90 &&
      lon >= -180 &&
      lon <= 180;
}

/// Settings and state kept between runs. Everything stays on the device.
class Settings {
  Settings(this._prefs);

  final SharedPreferences _prefs;

  /// 20 nautical miles (D15).
  static const defaultRadiusKm = 37.04;

  /// This install's primary aircraft source, chosen once at random (D17).
  Source? get primarySource {
    final name = _prefs.getString('primarySource');
    return Source.values.where((s) => s.name == name).firstOrNull;
  }

  Future<void> setPrimarySource(Source source) =>
      _prefs.setString('primarySource', source.name);

  /// Requests sent today, so restarts don't reset the daily count (D15).
  Usage? get usage {
    final day = _prefs.getInt('usage.day');
    if (day == null) return null;
    return Usage(
      day: day,
      adsbLol: _prefs.getInt('usage.adsbLol') ?? 0,
      adsbFi: _prefs.getInt('usage.adsbFi') ?? 0,
    );
  }

  Future<void> setUsage(Usage usage) async {
    await _prefs.setInt('usage.day', usage.day);
    await _prefs.setInt('usage.adsbLol', usage.adsbLol);
    await _prefs.setInt('usage.adsbFi', usage.adsbFi);
  }

  LocationMode get locationMode => _prefs.getString('location.mode') == 'manual'
      ? LocationMode.manual
      : LocationMode.device;

  Future<void> setLocationMode(LocationMode mode) =>
      _prefs.setString('location.mode', mode.name);

  LatLon? get manualLocation {
    final lat = _prefs.getDouble('location.lat');
    final lon = _prefs.getDouble('location.lon');
    return lat == null || lon == null ? null : LatLon(lat, lon);
  }

  Future<void> setManualLocation(LatLon place) async {
    await _prefs.setDouble('location.lat', place.lat);
    await _prefs.setDouble('location.lon', place.lon);
  }

  double get radiusKm => _prefs.getDouble('radiusKm') ?? defaultRadiusKm;

  /// Temperatures in °F instead of °C. Metric by default.
  bool get imperial => _prefs.getBool('units.imperial') ?? false;

  Future<void> setImperial(bool imperial) =>
      _prefs.setBool('units.imperial', imperial);

  AmbientTheme get ambientTheme =>
      _prefs.getString('ambient.theme') == AmbientTheme.ledMatrix.name
      ? AmbientTheme.ledMatrix
      : AmbientTheme.nightSky;

  Future<void> setAmbientTheme(AmbientTheme theme) =>
      _prefs.setString('ambient.theme', theme.name);

  /// When ambient mode dims itself; 23:00–07:00 by default.
  DimSchedule get dimSchedule => DimSchedule(
    enabled: _prefs.getBool('dim.enabled') ?? true,
    startMinutes: _prefs.getInt('dim.start') ?? 23 * 60,
    endMinutes: _prefs.getInt('dim.end') ?? 7 * 60,
  );

  Future<void> setDimSchedule(DimSchedule schedule) async {
    await _prefs.setBool('dim.enabled', schedule.enabled);
    await _prefs.setInt('dim.start', schedule.startMinutes);
    await _prefs.setInt('dim.end', schedule.endMinutes);
  }
}
