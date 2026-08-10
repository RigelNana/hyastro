use hyastro::{
    astro::Astrometry,
    earth::{Earth, EllipsoidalHeight, GeodeticLatitude, GeodeticLongitude, GeodeticPosition},
    ephem::{EphemerisProvider, SofaAnalyticEphemeris},
    event::{Events, LunarEclipseSearchOptions, LunarEclipseVisibilityOptions},
    time::{
        DateTime, Duration, EarthOrientationAcceptance, EarthOrientationTable, Gregorian, IersC04,
        ModifiedJulianDate, TimeContext, TimeInterval, Utc,
    },
};

#[cfg(feature = "anise")]
use hyastro::ephem::{Ephemeris, KernelManifest};

fn calculate<P: EphemerisProvider + ?Sized>(
    time: &TimeContext<'_, EarthOrientationTable<'_>>,
    ephemeris: &P,
) -> Result<(), Box<dyn std::error::Error>> {
    let earth = Earth::wgs84();
    let dallas = earth.fixed_site(
        "Dallas, Texas",
        GeodeticPosition::new(
            GeodeticLongitude::try_from_degrees(-96.7970)?,
            GeodeticLatitude::try_from_degrees(32.7767)?,
            EllipsoidalHeight::from_metres(131.0)?,
        ),
    )?;
    let events = Events::new(Astrometry::new(time, ephemeris));
    let start = time.resolve(DateTime::<Gregorian, Utc>::from_components(
        2022, 1, 1, 0, 0, 0, 0,
    )?)?;
    let end = time.resolve(DateTime::<Gregorian, Utc>::from_components(
        2023, 1, 1, 0, 0, 0, 0,
    )?)?;
    let eclipses = events.global_lunar_eclipses_in(
        &earth,
        TimeInterval::new(start, end)?,
        LunarEclipseSearchOptions::standard(),
    )?;

    for eclipse in eclipses {
        let maximum = eclipse.maximum();
        println!(
            "{:?} {:?} umbral={:+.4} penumbral={:+.4} axis={:.1} km PA={}",
            time.represent::<Gregorian, Utc>(maximum.instant())?,
            eclipse.kind(),
            maximum.geometry().umbral_magnitude().as_ratio(),
            maximum.geometry().penumbral_magnitude().as_ratio(),
            maximum.geometry().axis_distance().as_kilometres(),
            maximum
                .geometry()
                .position_angle()
                .map(|angle| format!("{:.1} deg", angle.as_degrees()))
                .unwrap_or_else(|| "on axis".to_owned()),
        );
        println!(
            "  model={} ephemeris={}",
            eclipse.model().shadow().identifier(),
            eclipse.ephemeris_provenance().model(),
        );
        let contacts = [
            Some(eclipse.penumbral_ingress()),
            eclipse.umbral_ingress(),
            eclipse.totality_ingress(),
            eclipse.totality_egress(),
            eclipse.umbral_egress(),
            Some(eclipse.penumbral_egress()),
        ];
        for contact in contacts.into_iter().flatten() {
            println!(
                "  {:?} {:?} PA={}",
                contact.kind(),
                time.represent::<Gregorian, Utc>(contact.instant())?,
                contact
                    .geometry()
                    .position_angle()
                    .map(|angle| format!("{:.1} deg", angle.as_degrees()))
                    .unwrap_or_else(|| "on axis".to_owned()),
            );
        }
        println!(
            "  durations penumbral={:.1} min partial={} total={}",
            eclipse.penumbral_phase()?.duration()?.as_seconds_f64() / 60.0,
            eclipse
                .partial_phase()?
                .map(|phase| format!(
                    "{:.1} min",
                    phase.duration().unwrap().as_seconds_f64() / 60.0
                ))
                .unwrap_or_else(|| "none".to_owned()),
            eclipse
                .total_phase()?
                .map(|phase| format!(
                    "{:.1} min",
                    phase.duration().unwrap().as_seconds_f64() / 60.0
                ))
                .unwrap_or_else(|| "none".to_owned()),
        );

        let visibility = events.local_lunar_eclipse_visibility(
            &dallas,
            &eclipse,
            LunarEclipseVisibilityOptions::standard(eclipse.model()),
        )?;
        println!(
            "  Dallas visibility={:?} low-altitude-warning={}",
            visibility.horizon_events().visibility(),
            visibility.has_low_altitude_warning(),
        );
        for sample in visibility.samples() {
            let horizontal = sample
                .observed_place()
                .map(|place| place.horizontal())
                .unwrap_or_else(|| sample.vacuum_place().horizontal());
            println!(
                "    {:?} altitude={:+.1} deg above={} low={} Sun={:+.1} deg {:?}",
                sample.stage(),
                horizontal.altitude().as_degrees(),
                sample.is_above_horizon(),
                sample.is_low_altitude(),
                sample.solar_altitude().as_degrees(),
                sample.sky_background(),
            );
        }
        for phase in visibility.visible_phases() {
            println!(
                "    visible {:?}: {:?} -> {:?} clipped={}/{}",
                phase.kind(),
                time.represent::<Gregorian, Utc>(phase.interval().start())?,
                time.represent::<Gregorian, Utc>(phase.interval().end())?,
                phase.is_truncated_at_start(),
                phase.is_truncated_at_end(),
            );
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = TimeContext::builtin();
    let data = IersC04::parse(include_str!(
        "../data/eop/eop-20u24-c04-1962-now-2026-08-06.txt"
    ))?;
    let samples = data.try_samples_in(
        &base,
        ModifiedJulianDate::<Utc>::from_parts(59_580.0, 0.0)?,
        ModifiedJulianDate::<Utc>::from_parts(59_947.0, 0.0)?,
        EarthOrientationAcceptance::FinalOnly,
    )?;
    let expires = samples[samples.len() - 1]
        .epoch()
        .checked_add(Duration::from_days(1)?)?;
    let eop = EarthOrientationTable::new(&samples, "IERS C04 2022 example", expires)?;
    let time = base.with_earth_orientation(eop);

    if let Some(kernel_path) = std::env::args_os().nth(1) {
        #[cfg(feature = "anise")]
        {
            let ephemeris = Ephemeris::load(KernelManifest::inspect([kernel_path])?)?;
            return calculate(&time, &ephemeris);
        }
        #[cfg(not(feature = "anise"))]
        {
            let _ = kernel_path;
            return Err("BSP input requires the `anise` feature".into());
        }
    }

    calculate(&time, &SofaAnalyticEphemeris::new())
}
