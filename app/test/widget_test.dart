import 'package:flutter_test/flutter_test.dart';
import 'package:passingover/main.dart';

void main() {
  testWidgets('shows the name, the tagline and the greeting', (tester) async {
    await tester.pumpWidget(const PassingOverApp(greeting: 'Hello from test'));
    expect(find.text('Passing Over'), findsOneWidget);
    expect(find.text('Where the planes above you are going.'), findsOneWidget);
    expect(find.text('Hello from test'), findsOneWidget);
  });
}
