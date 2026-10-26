import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:passingover/src/location.dart';
import 'package:passingover/src/now_screen.dart';
import 'package:passingover/src/rust/api/sky.dart';
import 'package:passingover/src/settings.dart';
import 'package:passingover/src/sky_controller.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'support/fakes.dart';

Future<SkyController> started(
  WidgetTester tester, {
  List<PollResult> answers = const [],
  FakeLocation? location,
}) async {
  SharedPreferences.setMockInitialValues({});
  final api = FakeSkyApi()..answers.addAll(answers);
  final controller = SkyController(
    api: api,
    settings: Settings(await SharedPreferences.getInstance()),
    location: location ?? FakeLocation(),
  );
  await tester.pumpWidget(MaterialApp(home: NowScreen(controller: controller)));
  await controller.start();
  await tester.pumpAndSettle();
  return controller;
}

void main() {
  testWidgets('shows where the flights are going', (tester) async {
    final controller = await started(
      tester,
      answers: [
        PollResult(view: viewOf([sk945, unknown])),
      ],
    );
    expect(find.text('SK 945'), findsOneWidget);
    expect(find.text('→ Chicago'), findsOneWidget);
    expect(
      find.text('15:12 there · landing in about 7 h 40 min'),
      findsOneWidget,
    );
    expect(find.text('from Stockholm'), findsOneWidget);
    expect(find.text('SAS1225'), findsOneWidget);
    expect(find.text('Destination unknown'), findsOneWidget);
    expect(find.text('Aircraft data: adsb.lol (ODbL)'), findsOneWidget);
    controller.dispose();
  });

  testWidgets('a quiet sky says so', (tester) async {
    final controller = await started(
      tester,
      answers: [PollResult(view: viewOf([]))],
    );
    expect(find.text('The sky above you is quiet right now.'), findsOneWidget);
    controller.dispose();
  });

  testWidgets('an error is calm and explains itself', (tester) async {
    final controller = await started(
      tester,
      answers: [const PollResult(error: 'adsb.lol: HTTP 503')],
    );
    expect(find.text('Can\'t see the sky right now.'), findsOneWidget);
    expect(find.text('adsb.lol: HTTP 503'), findsOneWidget);
    controller.dispose();
  });

  testWidgets('without a location it asks for one', (tester) async {
    final controller = await started(
      tester,
      location: FakeLocation(
        const LocationResult.problem('Location services are turned off.'),
      ),
    );
    expect(find.text('Where are you?'), findsOneWidget);
    expect(find.text('Location services are turned off.'), findsOneWidget);
    await tester.tap(find.text('Set location'));
    await tester.pumpAndSettle();
    expect(find.text('Or enter coordinates'), findsOneWidget);
    controller.dispose();
  });
}
