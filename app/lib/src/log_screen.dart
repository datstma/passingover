import 'package:flutter/material.dart';

import 'sky_controller.dart';

const _bright = Color(0xEBFFFFFF);
const _soft = Color(0xB3FFFFFF);
const _faint = Color(0x80FFFFFF);

/// Where the planes that passed over were going (M5, D38).
class LogScreen extends StatefulWidget {
  const LogScreen({super.key, required this.controller});

  final SkyController controller;

  @override
  State<LogScreen> createState() => _LogScreenState();
}

class _LogScreenState extends State<LogScreen> {
  LogPeriod _period = LogPeriod.month;
  late Future<List<String>> _summary = widget.controller.logSummary(_period);

  void _refresh() {
    setState(() {
      _summary = widget.controller.logSummary(_period);
    });
  }

  Future<void> _confirmClear() async {
    final clear = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Clear the daydream log?'),
        content: const Text(
          'Everything it remembers is deleted from this device.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Keep'),
          ),
          TextButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('Clear'),
          ),
        ],
      ),
    );
    if (clear == true) {
      await widget.controller.clearLog();
      _refresh();
    }
  }

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    final keep = widget.controller.keepLog;
    return Scaffold(
      backgroundColor: const Color(0xFF0B1426),
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        foregroundColor: _soft,
        title: Text(
          'Daydream log',
          style: text.titleLarge?.copyWith(
            fontWeight: FontWeight.w300,
            color: _bright,
          ),
        ),
      ),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(28, 8, 28, 28),
        children: [
          SegmentedButton<LogPeriod>(
            segments: [
              for (final p in LogPeriod.values)
                ButtonSegment(value: p, label: Text(p.label)),
            ],
            selected: {_period},
            showSelectedIcon: false,
            onSelectionChanged: (s) {
              _period = s.first;
              _refresh();
            },
          ),
          const SizedBox(height: 28),
          FutureBuilder<List<String>>(
            future: _summary,
            builder: (context, snapshot) {
              final lines = snapshot.data;
              if (lines == null) return const SizedBox(height: 120);
              if (!keep && lines.isEmpty) {
                return Text(
                  'The daydream log is off.',
                  style: text.headlineSmall?.copyWith(
                    fontWeight: FontWeight.w300,
                    color: _bright,
                  ),
                );
              }
              return Column(
                crossAxisAlignment: .start,
                children: [
                  for (final (i, line) in lines.indexed) ...[
                    Text(
                      line,
                      style: (i == 0 ? text.headlineSmall : text.titleMedium)
                          ?.copyWith(
                            fontWeight: FontWeight.w300,
                            color: i == 0 ? _bright : _soft,
                            height: 1.35,
                          ),
                    ),
                    const SizedBox(height: 16),
                  ],
                ],
              );
            },
          ),
          const SizedBox(height: 24),
          SwitchListTile(
            contentPadding: EdgeInsets.zero,
            title: const Text('Keep a daydream log'),
            subtitle: const Text(
              'Remembers where the planes overhead were going. It stays on '
              'this device and never includes your location.',
            ),
            value: keep,
            onChanged: (on) async {
              await widget.controller.setKeepLog(on);
              _refresh();
            },
          ),
          Align(
            alignment: .centerLeft,
            child: TextButton.icon(
              icon: const Icon(Icons.delete_outline),
              label: const Text('Clear the log'),
              style: TextButton.styleFrom(foregroundColor: _faint),
              onPressed: _confirmClear,
            ),
          ),
        ],
      ),
    );
  }
}
