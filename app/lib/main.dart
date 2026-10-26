import 'dart:async';
import 'dart:math';

import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:window_manager/window_manager.dart';

import 'src/ambient/ambient_platform.dart';
import 'src/home_screen_widget.dart';
import 'src/location.dart';
import 'src/menu_bar.dart';
import 'src/now_screen.dart';
import 'src/rust/api/sky.dart';
import 'src/rust/frb_generated.dart';
import 'src/settings.dart';
import 'src/sky_api.dart';
import 'src/sky_controller.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  if (DeviceAmbientPlatform.isDesktop) {
    // Lets ambient mode fill the screen on desktops (D36).
    await windowManager.ensureInitialized();
  }
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
  final navigatorKey = GlobalKey<NavigatorState>();
  runApp(PassingOverApp(controller: controller, navigatorKey: navigatorKey));
  if (MacMenuBar.supported) {
    MacMenuBar(controller: controller, navigatorKey: navigatorKey).start();
  }
  if (HomeScreenWidget.supported) HomeScreenWidget(controller).start();
  unawaited(controller.start());
}

class PassingOverApp extends StatefulWidget {
  const PassingOverApp({
    super.key,
    required this.controller,
    this.navigatorKey,
  });

  final SkyController controller;
  final GlobalKey<NavigatorState>? navigatorKey;

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
      navigatorKey: widget.navigatorKey,
      title: 'Passing Over',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        fontFamily: 'Overpass',
        colorScheme: .fromSeed(
          seedColor: const Color(0xFF1B2A4A),
          brightness: Brightness.dark,
        ),
      ),
      home: NowScreen(controller: widget.controller),
    );
  }
}
