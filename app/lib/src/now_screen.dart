import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

import 'rust/api/sky.dart';
import 'sheets.dart';
import 'sky_controller.dart';

const _bright = Color(0xEBFFFFFF);
const _soft = Color(0xB3FFFFFF);
const _faint = Color(0x80FFFFFF);
const _fainter = Color(0x5CFFFFFF);

/// The flights passing over right now: calm, few numbers (M2).
class NowScreen extends StatelessWidget {
  const NowScreen({super.key, required this.controller});

  final SkyController controller;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: DecoratedBox(
        decoration: const BoxDecoration(
          gradient: LinearGradient(
            begin: .topCenter,
            end: .bottomCenter,
            colors: [Color(0xFF0B1426), Color(0xFF1C2B4A)],
          ),
        ),
        child: SafeArea(
          child: ListenableBuilder(
            listenable: controller,
            builder: (context, _) => Column(
              crossAxisAlignment: .stretch,
              children: [
                _Header(controller: controller),
                Expanded(child: _Body(controller: controller)),
                _Footer(controller: controller),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _Header extends StatelessWidget {
  const _Header({required this.controller});

  final SkyController controller;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(28, 16, 12, 8),
      child: Row(
        children: [
          Expanded(
            child: Text(
              'Passing Over',
              style: Theme.of(context).textTheme.titleLarge?.copyWith(
                fontWeight: FontWeight.w300,
                letterSpacing: 1.2,
                color: _bright,
              ),
            ),
          ),
          IconButton(
            tooltip: 'Location and units',
            color: _soft,
            icon: const Icon(Icons.place_outlined),
            onPressed: () => showLocationSheet(context, controller),
          ),
          IconButton(
            tooltip: 'About the data',
            color: _soft,
            icon: const Icon(Icons.info_outline),
            onPressed: () => showAboutDataSheet(context),
          ),
        ],
      ),
    );
  }
}

class _Body extends StatelessWidget {
  const _Body({required this.controller});

  final SkyController controller;

  @override
  Widget build(BuildContext context) {
    final view = controller.view;
    return switch (controller.phase) {
      SkyPhase.needsLocation => _Message(
        title: 'Where are you?',
        detail:
            controller.locationProblem ??
            'Passing Over needs to know roughly where you are.',
        action: FilledButton.tonal(
          onPressed: () => showLocationSheet(context, controller),
          child: const Text('Set location'),
        ),
      ),
      _ when view == null && controller.error != null => _Message(
        title: 'Can\'t see the sky right now.',
        detail: controller.error,
      ),
      _ when view == null => const _Message(title: 'Looking up…'),
      _ when view.flights.isEmpty => const _Message(
        title: 'The sky above you is quiet right now.',
      ),
      _ => ListView.separated(
        padding: const EdgeInsets.fromLTRB(28, 16, 28, 24),
        itemCount: view.flights.length,
        separatorBuilder: (_, _) => const SizedBox(height: 28),
        itemBuilder: (context, i) => _FlightTile(
          key: ValueKey(view.flights[i].key),
          flight: view.flights[i],
        ),
      ),
    };
  }
}

class _FlightTile extends StatelessWidget {
  const _FlightTile({super.key, required this.flight});

  final FlightView flight;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    final there = [
      if (flight.localTime != null) '${flight.localTime} there',
      ?flight.weather,
    ].join(', ');
    final landing = flight.landingIn == null
        ? null
        : flight.landsNearby
        ? 'Landing nearby in ${flight.landingIn}'
        : 'Landing in ${flight.landingIn}';
    final journey =
        flight.journey ??
        (flight.origin == null ? null : 'from ${flight.origin}');
    return TweenAnimationBuilder<double>(
      tween: Tween(begin: 0, end: 1),
      duration: const Duration(milliseconds: 600),
      curve: Curves.easeOut,
      builder: (context, t, child) => Opacity(opacity: t, child: child),
      child: Semantics(
        label: [flight.line, ?flight.journey].join(' '),
        excludeSemantics: true,
        child: Column(
          crossAxisAlignment: .start,
          children: [
            Text(
              flight.title,
              style: text.labelLarge?.copyWith(
                letterSpacing: 1.5,
                color: _faint,
              ),
            ),
            const SizedBox(height: 4),
            _Destination(name: flight.destination, style: text.headlineMedium),
            if (there.isNotEmpty) ...[
              const SizedBox(height: 6),
              Text(there, style: text.bodyLarge?.copyWith(color: _soft)),
            ],
            if (landing != null) ...[
              const SizedBox(height: 2),
              Text(landing, style: text.bodyLarge?.copyWith(color: _soft)),
            ],
            if (journey != null) ...[
              const SizedBox(height: 6),
              Text(journey, style: text.bodyMedium?.copyWith(color: _fainter)),
            ],
          ],
        ),
      ),
    );
  }
}

/// "→ Chicago". The arrow is an icon: not every font has the glyph.
class _Destination extends StatelessWidget {
  const _Destination({required this.name, required this.style});

  final String? name;
  final TextStyle? style;

  @override
  Widget build(BuildContext context) {
    final base = style?.copyWith(fontWeight: FontWeight.w300);
    if (name == null) {
      return Text('Destination unknown', style: base?.copyWith(color: _faint));
    }
    return Text.rich(
      TextSpan(
        children: [
          WidgetSpan(
            alignment: PlaceholderAlignment.middle,
            child: Padding(
              padding: const EdgeInsets.only(right: 10),
              child: Icon(
                Icons.east,
                size: (base?.fontSize ?? 28) * 0.8,
                color: _soft,
              ),
            ),
          ),
          TextSpan(text: name),
        ],
      ),
      style: base?.copyWith(color: _bright),
    );
  }
}

class _Message extends StatelessWidget {
  const _Message({required this.title, this.detail, this.action});

  final String title;
  final String? detail;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 28),
      child: Column(
        mainAxisAlignment: .center,
        crossAxisAlignment: .start,
        children: [
          Text(
            title,
            style: text.headlineSmall?.copyWith(
              fontWeight: FontWeight.w300,
              color: _bright,
            ),
          ),
          if (detail != null) ...[
            const SizedBox(height: 12),
            Text(detail!, style: text.bodyLarge?.copyWith(color: _soft)),
          ],
          if (action != null) ...[const SizedBox(height: 20), action!],
        ],
      ),
    );
  }
}

class _Footer extends StatelessWidget {
  const _Footer({required this.controller});

  final SkyController controller;

  @override
  Widget build(BuildContext context) {
    final style = Theme.of(context).textTheme.bodySmall
        ?.copyWith(color: _fainter);
    final view = controller.view;
    final updated = controller.lastUpdate;
    return Padding(
      padding: const EdgeInsets.fromLTRB(28, 4, 28, 16),
      child: Wrap(
        spacing: 12,
        runSpacing: 4,
        crossAxisAlignment: .center,
        children: [
          if (updated != null)
            Text(
              'Updated ${MaterialLocalizations.of(context).formatTimeOfDay(TimeOfDay.fromDateTime(updated), alwaysUse24HourFormat: true)}',
              style: style,
            ),
          if (view != null)
            for (final credit in view.credits)
              InkWell(
                onTap: () => launchUrl(Uri.parse(credit.url)),
                child: Text(
                  credit.text,
                  style: style?.copyWith(decoration: TextDecoration.underline),
                ),
              ),
        ],
      ),
    );
  }
}
