import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

import 'rust/api/sky.dart';
import 'menu_bar.dart';
import 'settings.dart';
import 'sky_controller.dart';

/// Choose between the device's location and coordinates typed in.
Future<void> showLocationSheet(BuildContext context, SkyController controller) {
  return showModalBottomSheet(
    context: context,
    isScrollControlled: true,
    showDragHandle: true,
    builder: (context) => _LocationSheet(controller: controller),
  );
}

class _LocationSheet extends StatefulWidget {
  const _LocationSheet({required this.controller});

  final SkyController controller;

  @override
  State<_LocationSheet> createState() => _LocationSheetState();
}

class _LocationSheetState extends State<_LocationSheet> {
  late final TextEditingController _lat;
  late final TextEditingController _lon;
  String? _problem;

  @override
  void initState() {
    super.initState();
    final known = widget.controller.manualLocation ?? widget.controller.here;
    _lat = TextEditingController(text: known?.lat.toStringAsFixed(4) ?? '');
    _lon = TextEditingController(text: known?.lon.toStringAsFixed(4) ?? '');
  }

  @override
  void dispose() {
    _lat.dispose();
    _lon.dispose();
    super.dispose();
  }

  void _useCoordinates() {
    final place = LatLon(
      double.tryParse(_lat.text.trim().replaceAll(',', '.')) ?? double.nan,
      double.tryParse(_lon.text.trim().replaceAll(',', '.')) ?? double.nan,
    );
    if (!place.isValid) {
      setState(
        () => _problem = 'Latitude is −90 to 90 and longitude −180 to 180, in decimal degrees.',
      );
      return;
    }
    Navigator.of(context).pop();
    widget.controller.useManualLocation(place);
  }

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    final usingDevice = widget.controller.locationMode == LocationMode.device;
    return Padding(
      padding: EdgeInsets.fromLTRB(
        24,
        0,
        24,
        24 + MediaQuery.viewInsetsOf(context).bottom,
      ),
      child: Column(
        mainAxisSize: .min,
        crossAxisAlignment: .stretch,
        children: [
          Text('Location and units', style: text.titleLarge),
          const SizedBox(height: 8),
          Text(
            'Your location stays on this device. Only a point rounded to about '
            '1 km is sent to the aircraft data source.',
            style: text.bodyMedium,
          ),
          const SizedBox(height: 16),
          FilledButton.tonalIcon(
            icon: const Icon(Icons.my_location),
            label: Text(
              usingDevice
                  ? 'Using this device\'s location'
                  : 'Use this device\'s location',
            ),
            onPressed: () {
              Navigator.of(context).pop();
              widget.controller.useDeviceLocation();
            },
          ),
          const SizedBox(height: 24),
          Text('Or enter coordinates', style: text.titleSmall),
          const SizedBox(height: 8),
          Row(
            children: [
              Expanded(
                child: TextField(
                  controller: _lat,
                  decoration: const InputDecoration(labelText: 'Latitude'),
                  keyboardType: const TextInputType.numberWithOptions(
                    signed: true,
                    decimal: true,
                  ),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: TextField(
                  controller: _lon,
                  decoration: const InputDecoration(labelText: 'Longitude'),
                  keyboardType: const TextInputType.numberWithOptions(
                    signed: true,
                    decimal: true,
                  ),
                  onSubmitted: (_) => _useCoordinates(),
                ),
              ),
            ],
          ),
          if (_problem != null) ...[
            const SizedBox(height: 8),
            Text(
              _problem!,
              style: text.bodySmall?.copyWith(
                color: Theme.of(context).colorScheme.error,
              ),
            ),
          ],
          const SizedBox(height: 12),
          Align(
            alignment: .centerRight,
            child: TextButton(
              onPressed: _useCoordinates,
              child: const Text('Use these coordinates'),
            ),
          ),
          if (MacMenuBar.supported)
            SwitchListTile(
              contentPadding: EdgeInsets.zero,
              title: const Text('Show in the menu bar'),
              subtitle: const Text(
                'Keeps an eye on the sky (every minute) while the window is '
                'closed.',
              ),
              value: widget.controller.showInMenuBar,
              onChanged: (on) async {
                await widget.controller.setShowInMenuBar(on);
                if (mounted) setState(() {});
              },
            ),
          const SizedBox(height: 16),
          Text('Temperatures', style: text.titleSmall),
          const SizedBox(height: 8),
          SegmentedButton<bool>(
            segments: const [
              ButtonSegment(value: false, label: Text('°C')),
              ButtonSegment(value: true, label: Text('°F')),
            ],
            selected: {widget.controller.imperial},
            showSelectedIcon: false,
            onSelectionChanged: (selection) async {
              await widget.controller.setImperial(selection.first);
              if (mounted) setState(() {});
            },
          ),
        ],
      ),
    );
  }
}

/// Every data source, with the links their terms ask for.
Future<void> showAboutDataSheet(BuildContext context) {
  final credits = dataCredits();
  return showModalBottomSheet(
    context: context,
    showDragHandle: true,
    builder: (context) {
      final text = Theme.of(context).textTheme;
      return ListView(
        shrinkWrap: true,
        padding: const EdgeInsets.fromLTRB(24, 0, 24, 24),
        children: [
          Text('About the data', style: text.titleLarge),
          const SizedBox(height: 8),
          Text(
            'Passing Over is free and open source. It is built on community '
            'and open data; thank you to everyone who feeds and maintains it.',
            style: text.bodyMedium,
          ),
          const SizedBox(height: 8),
          for (final credit in credits)
            ListTile(
              contentPadding: EdgeInsets.zero,
              title: Text(credit.text),
              subtitle: Text(credit.url),
              trailing: const Icon(Icons.open_in_new, size: 18),
              onTap: () => launchUrl(Uri.parse(credit.url)),
            ),
          const SizedBox(height: 8),
          Text(
            'Your location is rounded to about 1 km before it is sent to the '
            'aircraft data source. Route lookups send only a flight\'s '
            'callsign. Times to landing are estimates.',
            style: text.bodySmall,
          ),
        ],
      );
    },
  );
}
