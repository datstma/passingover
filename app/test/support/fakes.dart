import 'package:passingover/src/location.dart';
import 'package:passingover/src/rust/api/sky.dart';
import 'package:passingover/src/settings.dart';
import 'package:passingover/src/sky_api.dart';

const sk945 = FlightView(
  key: '4ac9e2',
  title: 'SK 945',
  destination: 'Chicago',
  origin: 'Stockholm',
  localTime: '15:12',
  weather: '9°C and overcast',
  landingIn: 'about 7 h 40 min',
  landsNearby: false,
  journey:
      'Left Stockholm about 1 h 10 min ago and will cross the Norwegian Sea, '
      'Greenland and Canada.',
  line:
      'SK 945 → Chicago. It\'s 15:12 there, 9°C and overcast. '
      'Landing in about 7 h 40 min.',
  distanceKm: 4.2,
  altitudeM: 10668,
);

const aa43 = FlightView(
  key: 'a1b2c3',
  title: 'AA 43',
  destination: 'New York',
  origin: 'Paris',
  landingIn: 'about 5 min',
  landsNearby: true,
  journey: 'Left Paris about 7 h 40 min ago.',
  line: 'AA 43 → New York, landing nearby in about 5 min.',
  distanceKm: 3.0,
);

const unknown = FlightView(
  key: '45ac52',
  title: 'SAS1225',
  landsNearby: false,
  line: 'SAS1225, destination unknown.',
  distanceKm: 9.1,
);

NowView viewOf(List<FlightView> flights) => NowView(
  flights: flights,
  credits: const [
    Credit(text: 'Aircraft data: adsb.lol (ODbL)', url: 'https://adsb.lol'),
  ],
  source: Source.adsbLol,
  updatedUnixMs: 1792966614001,
);

/// Answers polls from a queue (default: one flight) and records them.
class FakeSkyApi implements SkyApi {
  final List<PollResult> answers = [];
  final List<({double lat, double lon, double radiusKm, bool imperial})> polls =
      [];
  Usage _usage = const Usage(day: 20386, adsbLol: 0, adsbFi: 0);

  @override
  Future<bool> loadBundledData() async => true;

  @override
  Future<PollResult> poll(
    double lat,
    double lon,
    double radiusKm, {
    required bool imperial,
  }) async {
    polls.add((lat: lat, lon: lon, radiusKm: radiusKm, imperial: imperial));
    _usage = Usage(day: _usage.day, adsbLol: _usage.adsbLol + 1, adsbFi: 0);
    return answers.isEmpty
        ? PollResult(view: viewOf([sk945]))
        : answers.removeAt(0);
  }

  @override
  Usage get usage => _usage;

  @override
  PollTiming get timing =>
      const PollTiming(intervalMs: 15000, jitterMs: 2000, minIntervalMs: 10000);
}

class FakeLocation implements LocationProvider {
  FakeLocation([
    this.result = const LocationResult.found(LatLon(59.65, 17.92)),
  ]);

  LocationResult result;
  int calls = 0;

  @override
  Future<LocationResult> locate() async {
    calls++;
    return result;
  }
}
