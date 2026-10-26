import 'package:flutter/material.dart';

import 'led_font.dart' as font;

const ledAmber = Color(0xFFFFB000);

/// Text drawn as dots on a fixed LED grid (D37). Lines wider than the
/// space scroll, one whole column at a time, like a real board.
class LedText extends StatefulWidget {
  const LedText(
    this.text, {
    super.key,
    required this.dotSize,
    this.color = ledAmber,
    this.columnsPerSecond = 12,
    this.margin = 2,
  });

  final String text;
  final double dotSize;
  final Color color;
  final double columnsPerSecond;

  /// Unlit columns before text that fits, so it doesn't touch the edge.
  final int margin;

  @override
  State<LedText> createState() => _LedTextState();
}

class _LedTextState extends State<LedText> with SingleTickerProviderStateMixin {
  late final AnimationController _scroll = AnimationController(vsync: this);

  static const _gapColumns = 12;

  String get _chars => font.ledText(widget.text);
  int get _columns => _chars.isEmpty ? 0 : _chars.length * 6 - 1;

  @override
  void dispose() {
    _scroll.dispose();
    super.dispose();
  }

  void _updateScroll(bool fits) {
    if (fits) {
      if (_scroll.isAnimating) _scroll.stop();
      return;
    }
    final seconds = (_columns + _gapColumns) / widget.columnsPerSecond;
    final duration = Duration(milliseconds: (seconds * 1000).round());
    if (_scroll.duration != duration || !_scroll.isAnimating) {
      _scroll.duration = duration;
      _scroll.repeat();
    }
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final dot = widget.dotSize;
        final visibleColumns = (constraints.maxWidth / dot).floor();
        final fits = _columns + 2 * widget.margin <= visibleColumns;
        _updateScroll(fits);
        final size = Size(visibleColumns * dot, font.glyphHeight * dot);
        return ClipRect(
          child: AnimatedBuilder(
            animation: _scroll,
            builder: (context, _) => CustomPaint(
              size: size,
              painter: _LedPainter(
                chars: _chars,
                dot: dot,
                color: widget.color,
                shift: fits
                    ? -widget.margin
                    : (_scroll.value * (_columns + _gapColumns)).floor(),
                loop: fits ? null : _columns + _gapColumns,
              ),
            ),
          ),
        );
      },
    );
  }
}

class _LedPainter extends CustomPainter {
  _LedPainter({
    required this.chars,
    required this.dot,
    required this.color,
    required this.shift,
    required this.loop,
  });

  final String chars;
  final double dot;
  final Color color;

  /// Columns the text has moved left.
  final int shift;

  /// Columns after which scrolling text repeats, if it scrolls.
  final int? loop;

  bool _lit(int column, int row) {
    var c = column + shift;
    if (loop != null) c %= loop!;
    if (c < 0) return false;
    final index = c ~/ 6;
    final col = c % 6;
    if (index >= chars.length || col == 5) return false;
    return font.dot(chars[index], col, row);
  }

  @override
  void paint(Canvas canvas, Size size) {
    final off = Paint()..color = color.withValues(alpha: 0.07);
    final on = Paint()..color = color;
    final glow = Paint()
      ..color = color.withValues(alpha: 0.25)
      ..maskFilter = MaskFilter.blur(BlurStyle.normal, dot * 0.35);
    final columns = (size.width / dot).floor();
    final radius = dot * 0.38;
    for (var row = 0; row < font.glyphHeight; row++) {
      for (var column = 0; column < columns; column++) {
        final center = Offset((column + 0.5) * dot, (row + 0.5) * dot);
        if (_lit(column, row)) {
          canvas.drawCircle(center, radius * 1.4, glow);
          canvas.drawCircle(center, radius, on);
        } else {
          canvas.drawCircle(center, radius, off);
        }
      }
    }
  }

  @override
  bool shouldRepaint(_LedPainter old) =>
      old.chars != chars || old.shift != shift || old.dot != dot;
}
