import 'dart:async';
import 'dart:math';

import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'src/location.dart';
import 'src/now_screen.dart';
import 'src/rust/api/sky.dart';
import 'src/rust/frb_generated.dart';
import 'src/settings.dart';
import 'src/sky_api.dart';
import 'src/sky_controller.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  final settings = Settings(await SharedPreferences.getInstance());
  final random = Random();

  // Each install keeps one randomly chosen primary source (D17).
  var primary = settings.primarySource;
  if (primary == null) {
    primary = choosePrimarySource(roll: random.nextDouble());
    await settings.setPrimarySource(primary);
  }
  final handle = SkyHandle(primary: primary, seed: random.nextInt(1 << 32));
  final usage = settings.usage;
  if (usage != null) handle.setUsage(usage: usage);

  final controller = SkyController(
    api: RustSkyApi(handle),
    settings: settings,
    location: DeviceLocation(),
    random: random,
  );
  runApp(PassingOverApp(controller: controller));
  unawaited(controller.start());
}

class PassingOverApp extends StatefulWidget {
  const PassingOverApp({super.key, required this.controller});

  final SkyController controller;

  @override
  State<PassingOverApp> createState() => _PassingOverAppState();
}

class _PassingOverAppState extends State<PassingOverApp> {
  late final AppLifecycleListener _lifecycle;

  @override
  void initState() {
    super.initState();
    // Don't poll while nobody can see the screen (D15).
    _lifecycle = AppLifecycleListener(
      onHide: widget.controller.pause,
      onShow: () => unawaited(widget.controller.resume()),
    );
  }

  @override
  void dispose() {
    _lifecycle.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Passing Over',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorScheme: .fromSeed(
          seedColor: const Color(0xFF1B2A4A),
          brightness: Brightness.dark,
        ),
      ),
      home: NowScreen(controller: widget.controller),
    );
  }
}
