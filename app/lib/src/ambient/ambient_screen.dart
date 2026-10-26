import 'dart:async';
import 'dart:math';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../rust/api/sky.dart';
import '../settings.dart';
import '../sky_controller.dart';
import 'ambient_platform.dart';
import 'led_text.dart';
import 'star_field.dart';

/// How long each flight stays on screen before the next one fades in.
const ambientCycle = Duration(seconds: 12);

/// A calm full-screen view for a desktop, a spare tablet or a TV (M4, D36):
/// one flight at a time, polled every minute, dimmed at night.
class AmbientScreen extends StatefulWidget {
  const AmbientScreen({
    super.key,
    required this.controller,
    this.platform = const DeviceAmbientPlatform(),
    this.clock = DateTime.now,
  });

  final SkyController controller;
  final AmbientPlatform platform;
  final DateTime Function() clock;

  @override
  State<AmbientScreen> createState() => _AmbientScreenState();
}

class _AmbientScreenState extends State<AmbientScreen> {
  late final Timer _cycle;
  late final Timer _minute;
  Timer? _hideControls;
  String? _currentKey;
  bool _controls = false;

  SkyController get _sky => widget.controller;
  Settings get _settings => _sky.settings;

  @override
  void initState() {
    super.initState();
    _sky.setAmbient(true);
    unawaited(widget.platform.enter());
    _cycle = Timer.periodic(ambientCycle, (_) => setState(_advance));
    // Keeps the dimming in step with the clock.
    _minute = Timer.periodic(
      const Duration(minutes: 1),
      (_) => setState(() {}),
    );
  }

  @override
  void dispose() {
    _cycle.cancel();
    _minute.cancel();
    _hideControls?.cancel();
    _sky.setAmbient(false);
    unawaited(widget.platform.exit());
    super.dispose();
  }

  List<FlightView> get _flights => _sky.view?.flights ?? const [];

  FlightView? get _current {
    final flights = _flights;
    if (flights.isEmpty) return null;
    return flights.firstWhere(
      (f) => f.key == _currentKey,
      orElse: () => flights.first,
    );
  }

  void _advance() {
    final flights = _flights;
    if (flights.isEmpty) return;
    final i = flights.indexWhere((f) => f.key == _currentKey);
    _currentKey = flights[(i + 1) % flights.length].key;
  }

  void _toggleControls() {
    setState(() => _controls = !_controls);
    _hideControls?.cancel();
    if (_controls) {
      _hideControls = Timer(
        const Duration(seconds: 8),
        () => setState(() => _controls = false),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.escape): () =>
            Navigator.of(context).maybePop(),
      },
      child: Focus(
        autofocus: true,
        child: Scaffold(
          backgroundColor: Colors.black,
          body: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTap: _toggleControls,
            child: ListenableBuilder(
              listenable: _sky,
              builder: (context, _) {
                final flight = _current;
                _currentKey = flight?.key;
                final brightness = _settings.dimSchedule.brightnessAt(
                  widget.clock(),
                );
                return Stack(
                  fit: StackFit.expand,
                  children: [
                    switch (_settings.ambientTheme) {
                      AmbientTheme.nightSky => _NightSky(
                        flight: flight,
                        view: _sky.view,
                      ),
                      AmbientTheme.ledMatrix => _LedBoard(flight: flight),
                    },
                    IgnorePointer(
                      child: AnimatedContainer(
                        duration: const Duration(seconds: 2),
                        color: Colors.black.withValues(alpha: 1 - brightness),
                      ),
                    ),
                    AnimatedOpacity(
                      opacity: _controls ? 1 : 0,
                      duration: const Duration(milliseconds: 300),
                      child: IgnorePointer(
                        ignoring: !_controls,
                        child: _Controls(
                          settings: _settings,
                          onChanged: () => setState(() {}),
                        ),
                      ),
                    ),
                  ],
                );
              },
            ),
          ),
        ),
      ),
    );
  }
}

/// Type that grows with the screen: phone to TV.
double _scale(BuildContext context) {
  final size = MediaQuery.sizeOf(context);
  return (min(size.width, size.height * 1.6) / 900).clamp(0.7, 2.4);
}

class _NightSky extends StatelessWidget {
  const _NightSky({required this.flight, required this.view});

  final FlightView? flight;
  final NowView? view;

  @override
  Widget build(BuildContext context) {
    final s = _scale(context);
    return DecoratedBox(
      decoration: const BoxDecoration(
        gradient: LinearGradient(
          begin: .topCenter,
          end: .bottomCenter,
          colors: [Color(0xFF03050C), Color(0xFF0B1426), Color(0xFF16233F)],
        ),
      ),
      child: Stack(
        fit: StackFit.expand,
        children: [
          const StarField(),
          SafeArea(
            child: Padding(
              padding: EdgeInsets.all(48 * s),
              child: Column(
                crossAxisAlignment: .start,
                children: [
                  const Spacer(),
                  AnimatedSwitcher(
                    duration: const Duration(milliseconds: 1500),
                    child: _NightFlight(
                      key: ValueKey(flight?.key ?? 'quiet'),
                      flight: flight,
                      scale: s,
                    ),
                  ),
                  const Spacer(),
                  if (view != null)
                    Text(
                      view!.credits.map((c) => c.text).join(' · '),
                      style: TextStyle(
                        fontSize: 11 * s,
                        color: const Color(0x4DFFFFFF),
                      ),
                    ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _NightFlight extends StatelessWidget {
  const _NightFlight({super.key, required this.flight, required this.scale});

  final FlightView? flight;
  final double scale;

  @override
  Widget build(BuildContext context) {
    final s = scale;
    TextStyle style(
      double size,
      double alpha, [
      FontWeight w = FontWeight.w300,
    ]) => TextStyle(
      fontSize: size * s,
      fontWeight: w,
      color: Colors.white.withValues(alpha: alpha),
      height: 1.2,
    );
    final f = flight;
    if (f == null) {
      return Text(
        'The sky above you is quiet right now.',
        style: style(34, 0.85, FontWeight.w200),
      );
    }
    final there = [
      if (f.localTime != null) '${f.localTime} there',
      ?f.weather,
    ].join(', ');
    final landing = f.landingIn == null
        ? null
        : f.landsNearby
        ? 'Landing nearby in ${f.landingIn}'
        : 'Landing in ${f.landingIn}';
    return Column(
      crossAxisAlignment: .start,
      mainAxisSize: .min,
      children: [
        Text(
          f.title,
          style: style(16, 0.55, FontWeight.w500).copyWith(letterSpacing: 2),
        ),
        SizedBox(height: 8 * s),
        Text(
          f.destination ?? 'Destination unknown',
          style: style(64, f.destination == null ? 0.5 : 0.95, FontWeight.w200),
        ),
        if (there.isNotEmpty) ...[
          SizedBox(height: 12 * s),
          Text(there, style: style(24, 0.75)),
        ],
        if (landing != null) Text(landing, style: style(24, 0.75)),
        if (f.journey != null) ...[
          SizedBox(height: 16 * s),
          ConstrainedBox(
            constraints: BoxConstraints(maxWidth: 720 * s),
            child: Text(f.journey!, style: style(17, 0.45)),
          ),
        ],
      ],
    );
  }
}

class _LedBoard extends StatelessWidget {
  const _LedBoard({required this.flight});

  final FlightView? flight;

  @override
  Widget build(BuildContext context) {
    final f = flight;
    final rows = <(String, Color)>[];
    if (f == null) {
      rows.add(('Quiet sky', ledAmber));
    } else {
      rows.add((
        f.destination == null ? f.title : '${f.title} → ${f.destination}',
        ledAmber,
      ));
      if (f.landsNearby) {
        rows.add(('Landing nearby', ledAmber));
      } else {
        final there = [
          ?f.localTime,
          ?f.weather?.replaceFirst(' and ', ' '),
        ].join('  ');
        if (there.isNotEmpty) rows.add((there, ledAmber));
      }
      if (f.landingIn != null) {
        // Terse, like a board: "LANDS IN 7 H 40 MIN".
        rows.add((
          'Lands in ${f.landingIn!.replaceFirst('about ', '')}',
          ledAmber,
        ));
      }
      if (f.journey != null) {
        rows.add((f.journey!, ledAmber.withValues(alpha: 0.6)));
      }
    }
    return ColoredBox(
      color: Colors.black,
      child: SafeArea(
        child: LayoutBuilder(
          builder: (context, constraints) {
            // About 22 characters across, so a TV row is readable from the sofa.
            final dot = constraints.maxWidth / (22 * 6);
            return Center(
              child: AnimatedSwitcher(
                duration: const Duration(milliseconds: 600),
                child: Column(
                  key: ValueKey(f?.key ?? 'quiet'),
                  mainAxisSize: .min,
                  children: [
                    for (final (text, color) in rows) ...[
                      LedText(text, dotSize: dot, color: color),
                      SizedBox(height: dot * 3),
                    ],
                  ],
                ),
              ),
            );
          },
        ),
      ),
    );
  }
}

class _Controls extends StatelessWidget {
  const _Controls({required this.settings, required this.onChanged});

  final Settings settings;
  final VoidCallback onChanged;

  @override
  Widget build(BuildContext context) {
    return Align(
      alignment: .bottomCenter,
      child: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Material(
            color: const Color(0xCC111827),
            borderRadius: BorderRadius.circular(16),
            child: Padding(
              padding: const EdgeInsets.all(12),
              child: Wrap(
                spacing: 12,
                runSpacing: 8,
                crossAxisAlignment: .center,
                children: [
                  SegmentedButton<AmbientTheme>(
                    segments: const [
                      ButtonSegment(
                        value: AmbientTheme.nightSky,
                        label: Text('Night sky'),
                      ),
                      ButtonSegment(
                        value: AmbientTheme.ledMatrix,
                        label: Text('LED'),
                      ),
                    ],
                    selected: {settings.ambientTheme},
                    showSelectedIcon: false,
                    onSelectionChanged: (s) async {
                      await settings.setAmbientTheme(s.first);
                      onChanged();
                    },
                  ),
                  TextButton.icon(
                    icon: const Icon(Icons.bedtime_outlined),
                    label: const Text('Dimming'),
                    onPressed: () async {
                      await showDimSheet(context, settings);
                      onChanged();
                    },
                  ),
                  TextButton.icon(
                    icon: const Icon(Icons.close),
                    label: const Text('Close'),
                    onPressed: () => Navigator.of(context).maybePop(),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// Turn night dimming on or off and choose when it starts and ends.
Future<void> showDimSheet(BuildContext context, Settings settings) {
  return showModalBottomSheet(
    context: context,
    showDragHandle: true,
    builder: (context) => StatefulBuilder(
      builder: (context, setState) {
        final schedule = settings.dimSchedule;
        String hhmm(int minutes) =>
            '${(minutes ~/ 60).toString().padLeft(2, '0')}:'
            '${(minutes % 60).toString().padLeft(2, '0')}';
        Future<void> pick(bool start) async {
          final current = start ? schedule.startMinutes : schedule.endMinutes;
          final picked = await showTimePicker(
            context: context,
            initialTime: TimeOfDay(hour: current ~/ 60, minute: current % 60),
          );
          if (picked == null) return;
          final minutes = picked.hour * 60 + picked.minute;
          await settings.setDimSchedule(
            start
                ? schedule.copyWith(startMinutes: minutes)
                : schedule.copyWith(endMinutes: minutes),
          );
          setState(() {});
        }

        return Padding(
          padding: const EdgeInsets.fromLTRB(24, 0, 24, 24),
          child: Column(
            mainAxisSize: .min,
            crossAxisAlignment: .stretch,
            children: [
              SwitchListTile(
                contentPadding: EdgeInsets.zero,
                title: const Text('Dim at night'),
                subtitle: const Text(
                  'Fades the screen to 30 % so it doesn\'t light up the room.',
                ),
                value: schedule.enabled,
                onChanged: (on) async {
                  await settings.setDimSchedule(schedule.copyWith(enabled: on));
                  setState(() {});
                },
              ),
              ListTile(
                contentPadding: EdgeInsets.zero,
                enabled: schedule.enabled,
                title: const Text('From'),
                trailing: Text(hhmm(schedule.startMinutes)),
                onTap: () => pick(true),
              ),
              ListTile(
                contentPadding: EdgeInsets.zero,
                enabled: schedule.enabled,
                title: const Text('Until'),
                trailing: Text(hhmm(schedule.endMinutes)),
                onTap: () => pick(false),
              ),
            ],
          ),
        );
      },
    ),
  );
}
