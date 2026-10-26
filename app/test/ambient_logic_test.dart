import 'package:flutter_test/flutter_test.dart';
import 'package:passingover/src/ambient/dimming.dart';
import 'package:passingover/src/ambient/led_font.dart';

void main() {
  group('dim schedule', () {
    const night = DimSchedule(
      enabled: true,
      startMinutes: 23 * 60,
      endMinutes: 7 * 60,
    );
    DateTime at(int h, int m) => DateTime(2026, 10, 26, h, m);

    test('bright by day, dim at night, fading over 30 minutes', () {
      expect(night.brightnessAt(at(22, 0)), 1);
      expect(night.brightnessAt(at(23, 0)), 1);
      expect(night.brightnessAt(at(23, 15)), closeTo(0.65, 1e-9));
      expect(night.brightnessAt(at(2, 0)), closeTo(0.3, 1e-9));
      expect(night.brightnessAt(at(6, 45)), closeTo(0.65, 1e-9));
      expect(night.brightnessAt(at(7, 0)), 1);
      expect(night.brightnessAt(at(12, 0)), 1);
    });

    test('can be turned off or have an empty window', () {
      expect(night.copyWith(enabled: false).brightnessAt(at(2, 0)), 1);
      expect(night.copyWith(endMinutes: 23 * 60).brightnessAt(at(23, 30)), 1);
    });

    test('works without crossing midnight', () {
      final afternoon = night.copyWith(
        startMinutes: 13 * 60,
        endMinutes: 15 * 60,
      );
      expect(afternoon.brightnessAt(at(14, 0)), closeTo(0.3, 1e-9));
      expect(afternoon.brightnessAt(at(16, 0)), 1);
    });
  });

  group('LED font', () {
    test('every glyph is 5 by 7', () {
      for (final c in ledCharacters) {
        for (var row = 0; row < glyphHeight; row++) {
          for (var col = 0; col < glyphWidth; col++) {
            dot(c, col, row);
          }
        }
      }
      expect(ledCharacters.length, greaterThan(45));
    });

    test('folds accents and drops what it cannot draw', () {
      expect(ledText('Malmö'), 'MALMO');
      expect(ledText('São Paulo'), 'SAO PAULO');
      expect(ledText('Zürich · 9°C'), 'ZURICH - 9°C');
      expect(ledText('Straße'), 'STRASSE');
      expect(ledText('東京'), '  ');
      expect(ledText('SK 945 → Chicago'), 'SK 945 → CHICAGO');
    });

    test('draws an A', () {
      expect(dot('A', 2, 0), isTrue);
      expect(dot('A', 0, 0), isFalse);
      expect(dot('A', 0, 3), isTrue);
    });
  });
}
