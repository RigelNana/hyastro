use hyastro::{
    astro::Astrometry,
    ephem::{CelestialBody, SofaAnalyticEphemeris},
    event::{
        AngularEventSearchOptions, AstrometricMode, ConfigurationCoordinate, ConfigurationKind,
        ConfigurationQuery, DistanceExtremumQuery, Events, ExtremumKind, ExtremumSearchOptions,
        RelativeBodyQuery, StationQuery,
    },
    time::{DateTime, Gregorian, TimeContext, TimeInterval, Utc},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let time = TimeContext::builtin();
    let ephemeris = SofaAnalyticEphemeris::new();
    let events = Events::new(Astrometry::new(&time, &ephemeris));
    let start = time.resolve(DateTime::<Gregorian, Utc>::from_components(
        2024, 1, 1, 0, 0, 0, 0,
    )?)?;
    let end = time.resolve(DateTime::<Gregorian, Utc>::from_components(
        2025, 1, 1, 0, 0, 0, 0,
    )?)?;
    let interval = TimeInterval::new(start, end)?;
    let options = AngularEventSearchOptions::standard();
    let extremum_options = ExtremumSearchOptions::standard();

    let mercury_sun = RelativeBodyQuery::new(
        CelestialBody::MercuryBarycenter,
        CelestialBody::Sun,
        AstrometricMode::Apparent,
    )?;
    let mercury_conjunctions = events.configurations_in(
        interval,
        ConfigurationQuery::new(
            mercury_sun,
            ConfigurationKind::Conjunction,
            ConfigurationCoordinate::EclipticLongitude,
        ),
        options,
    )?;
    let mercury_elongations =
        events.greatest_elongations_in(interval, mercury_sun, extremum_options)?;
    let mercury_stations = events.stations_in(
        interval,
        StationQuery::new(CelestialBody::MercuryBarycenter, AstrometricMode::Apparent),
        options,
    )?;

    let jupiter_sun = RelativeBodyQuery::new(
        CelestialBody::JupiterBarycenter,
        CelestialBody::Sun,
        AstrometricMode::Apparent,
    )?;
    let jupiter_oppositions = events.configurations_in(
        interval,
        ConfigurationQuery::new(
            jupiter_sun,
            ConfigurationKind::Opposition,
            ConfigurationCoordinate::EclipticLongitude,
        ),
        options,
    )?;
    let jupiter_perigees = events.distance_extrema_in(
        interval,
        DistanceExtremumQuery::new(
            CelestialBody::JupiterBarycenter,
            CelestialBody::Earth,
            ExtremumKind::Minimum,
        )?,
        extremum_options,
    )?;

    println!("ephemeris = {}", SofaAnalyticEphemeris::MODEL);
    println!("interval  = 2024-01-01T00:00:00Z..2025-01-01T00:00:00Z");
    println!(
        "Mercury conjunctions       = {}",
        mercury_conjunctions.len()
    );
    println!(
        "Mercury greatest elongations = {}",
        mercury_elongations.len()
    );
    for event in &mercury_elongations {
        let utc = time.represent::<Gregorian, Utc>(event.instant())?;
        println!(
            "  {:04}-{:02}-{:02}  {:?}, {:.6} deg",
            utc.date().year(),
            utc.date().month(),
            utc.date().day(),
            event.side(),
            event.separation().as_degrees(),
        );
    }
    println!("Mercury stations           = {}", mercury_stations.len());
    println!("Jupiter oppositions        = {}", jupiter_oppositions.len());
    for event in &jupiter_oppositions {
        let utc = time.represent::<Gregorian, Utc>(event.instant())?;
        println!(
            "  {:04}-{:02}-{:02}  separation {:.6} deg",
            utc.date().year(),
            utc.date().month(),
            utc.date().day(),
            event.separation().as_degrees(),
        );
    }
    println!("Jupiter distance minima    = {}", jupiter_perigees.len());
    for event in &jupiter_perigees {
        let utc = time.represent::<Gregorian, Utc>(event.instant())?;
        println!(
            "  {:04}-{:02}-{:02}  {:.6} au",
            utc.date().year(),
            utc.date().month(),
            utc.date().day(),
            event.distance().as_astronomical_units(),
        );
    }

    Ok(())
}
