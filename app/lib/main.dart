import 'package:flutter/material.dart';

import 'src/rust/api/hello.dart';
import 'src/rust/frb_generated.dart';

Future<void> main() async {
  await RustLib.init();
  runApp(PassingOverApp(greeting: hello()));
}

/// M0 placeholder: a calm screen that shows the Rust core is wired up.
/// The real "Now" screen arrives in M2.
class PassingOverApp extends StatelessWidget {
  const PassingOverApp({super.key, required this.greeting});

  /// Text from the Rust core, shown quietly at the bottom.
  final String greeting;

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
      home: _Sky(greeting: greeting),
    );
  }
}

class _Sky extends StatelessWidget {
  const _Sky({required this.greeting});

  final String greeting;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
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
          child: Padding(
            padding: const EdgeInsets.all(32),
            child: Column(
              crossAxisAlignment: .start,
              children: [
                const Spacer(),
                Text(
                  'Passing Over',
                  style: text.displaySmall?.copyWith(
                    fontWeight: FontWeight.w300,
                    letterSpacing: 1.2,
                    color: const Color(0xEBFFFFFF),
                  ),
                ),
                const SizedBox(height: 12),
                Text(
                  'Where the planes above you are going.',
                  style: text.titleMedium?.copyWith(
                    fontWeight: FontWeight.w300,
                    color: const Color(0xB3FFFFFF),
                  ),
                ),
                const Spacer(),
                Text(
                  greeting,
                  style: text.bodySmall?.copyWith(
                    color: const Color(0x61FFFFFF),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
