import 'dart:math';

import 'package:flutter/material.dart';

/// A quiet, slowly twinkling star field for the night-sky theme.
class StarField extends StatefulWidget {
  const StarField({super.key, this.count = 160});

  final int count;

  @override
  State<StarField> createState() => _StarFieldState();
}

class _StarFieldState extends State<StarField>
    with SingleTickerProviderStateMixin {
  late final AnimationController _twinkle = AnimationController(
    vsync: this,
    duration: const Duration(seconds: 12),
  )..repeat();

  // Seeded, so the sky looks the same every time.
  late final List<_Star> _stars = () {
    final random = Random(7);
    return List.generate(
      widget.count,
      (_) => _Star(
        x: random.nextDouble(),
        y: pow(random.nextDouble(), 1.4).toDouble(),
        radius: 0.4 + random.nextDouble() * 1.1,
        brightness: 0.25 + random.nextDouble() * 0.6,
        phase: random.nextDouble() * 2 * pi,
      ),
    );
  }();

  @override
  void dispose() {
    _twinkle.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return RepaintBoundary(
      child: CustomPaint(
        size: Size.infinite,
        painter: _StarPainter(_stars, _twinkle),
      ),
    );
  }
}

class _Star {
  const _Star({
    required this.x,
    required this.y,
    required this.radius,
    required this.brightness,
    required this.phase,
  });

  final double x, y, radius, brightness, phase;
}

class _StarPainter extends CustomPainter {
  _StarPainter(this.stars, this.twinkle) : super(repaint: twinkle);

  final List<_Star> stars;
  final Animation<double> twinkle;

  @override
  void paint(Canvas canvas, Size size) {
    final t = twinkle.value * 2 * pi;
    final paint = Paint();
    for (final s in stars) {
      final alpha = (s.brightness * (0.75 + 0.25 * sin(t + s.phase))).clamp(
        0.0,
        1.0,
      );
      paint.color = Colors.white.withValues(alpha: alpha);
      canvas.drawCircle(
        Offset(s.x * size.width, s.y * size.height),
        s.radius,
        paint,
      );
    }
  }

  @override
  bool shouldRepaint(_StarPainter old) => false;
}
