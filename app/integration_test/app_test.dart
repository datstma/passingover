import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:passingover/main.dart';
import 'package:passingover/src/rust/api/hello.dart';
import 'package:passingover/src/rust/frb_generated.dart';

/// Runs the real app with the Rust library loaded: proves the bridge works
/// on the platform the test runs on.
void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  testWidgets('shows the greeting from the Rust core', (tester) async {
    await tester.pumpWidget(PassingOverApp(greeting: hello()));
    expect(find.textContaining('Hello from passingover-core'), findsOneWidget);
    expect(find.text('Passing Over'), findsOneWidget);
  });
}
