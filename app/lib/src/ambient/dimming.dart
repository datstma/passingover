/// When ambient mode dims itself (M4, D36): a nightly window, entered and
/// left with a slow fade, so a screen on a shelf doesn't light up the room.
class DimSchedule {
  const DimSchedule({
    required this.enabled,
    required this.startMinutes,
    required this.endMinutes,
    this.level = 0.3,
    this.fade = const Duration(minutes: 30),
  });

  final bool enabled;

  /// Minutes after midnight, local time. The window may cross midnight.
  final int startMinutes;
  final int endMinutes;

  /// Brightness while dimmed, 0–1.
  final double level;
  final Duration fade;

  /// Brightness at `time`: 1 outside the window, [level] inside, fading
  /// over [fade] after the start and before the end.
  double brightnessAt(DateTime time) {
    if (!enabled) return 1;
    const day = 24 * 60;
    final window = (endMinutes - startMinutes) % day;
    if (window == 0) return 1;
    final now = time.hour * 60 + time.minute + time.second / 60;
    final sinceStart = (now - startMinutes) % day;
    if (sinceStart >= window) return 1;
    final fadeMinutes = fade.inSeconds / 60;
    final dim = [
      1.0,
      sinceStart / fadeMinutes,
      (window - sinceStart) / fadeMinutes,
    ].reduce((a, b) => a < b ? a : b);
    return 1 - dim * (1 - level);
  }

  DimSchedule copyWith({bool? enabled, int? startMinutes, int? endMinutes}) =>
      DimSchedule(
        enabled: enabled ?? this.enabled,
        startMinutes: startMinutes ?? this.startMinutes,
        endMinutes: endMinutes ?? this.endMinutes,
        level: level,
        fade: fade,
      );
}
