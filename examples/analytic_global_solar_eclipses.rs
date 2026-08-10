use hyastro::{
    astro::Astrometry,
    earth::Earth,
    ephem::{EphemerisProvider, SofaAnalyticEphemeris},
    event::{
        AngularEventSearchOptions, BesselianPolynomialOptions, Events,
        GlobalSolarEclipsePathOptions, SolarEclipseSearchOptions,
    },
    math::Angle,
    time::{
        DateTime, Duration, EarthOrientationAcceptance, EarthRotationTable, Gregorian,
        IersFinals2000A, ModifiedJulianDate, TimeContext, TimeInterval, Utc,
    },
};

#[cfg(feature = "anise")]
use hyastro::ephem::{Ephemeris, KernelManifest};

fn calculate<P: EphemerisProvider + ?Sized>(
    time: &TimeContext<'_, EarthRotationTable<'_>>,
    ephemeris: &P,
) -> Result<(), Box<dyn std::error::Error>> {
    let events = Events::new(Astrometry::new(time, ephemeris));
    let start = time.resolve(DateTime::<Gregorian, Utc>::from_components(
        2026, 1, 1, 0, 0, 0, 0,
    )?)?;
    let end = time.resolve(DateTime::<Gregorian, Utc>::from_components(
        2027, 1, 1, 0, 0, 0, 0,
    )?)?;
    let standard = AngularEventSearchOptions::standard();
    let search = AngularEventSearchOptions::new(
        standard.scan_step(),
        standard.time_tolerance(),
        Angle::from_radians(1.0e-10)?,
        standard.max_refinement_iterations(),
        standard.max_evaluations(),
        standard.light_time(),
    )?;
    let options =
        SolarEclipseSearchOptions::new(search, SolarEclipseSearchOptions::standard().model());
    let earth = Earth::wgs84();
    let eclipses =
        events.global_solar_eclipses_in(&earth, TimeInterval::new(start, end)?, options)?;

    for eclipse in eclipses {
        let maximum = eclipse.maximum();
        let polynomial = events.solar_eclipse_besselian_polynomial(
            &earth,
            maximum.instant(),
            BesselianPolynomialOptions::nasa_six_hour(),
        )?;
        let besselian = polynomial.elements_at(maximum.instant())?;
        println!(
            "{:?} {:?} gamma={:+.5} axis={:.1} km geometric-core={:+.1} km central={}",
            time.represent::<Gregorian, Utc>(maximum.instant())?,
            eclipse.kind(),
            maximum.gamma().as_equatorial_radii(),
            maximum.shadow_axis_distance().as_kilometres(),
            maximum
                .geometric_core_shadow_radius_at_axis_plane()
                .as_metres()
                / 1_000.0,
            eclipse.central_path().is_some(),
        );
        println!(
            "  Bessel x={:+.6} y={:+.6} d={:+.6}° mu={:.6}° l1={:+.6} l2={:+.6} contact-core={:+.1} km",
            besselian.x().as_equatorial_radii(),
            besselian.y().as_equatorial_radii(),
            besselian.d().as_degrees(),
            besselian.mu().as_degrees(),
            besselian.l1().as_equatorial_radii(),
            besselian.l2().as_equatorial_radii(),
            besselian
                .contact_core_shadow_radius_at_fundamental_plane()
                .as_metres()
                / 1_000.0,
        );
        println!(
            "  x(t)={:?}; max fit residual={:.3e}",
            polynomial.x().coefficients(),
            polynomial.residuals().x().as_equatorial_radii(),
        );
        if let Some(path) = eclipse.central_path() {
            println!(
                "  path {:?} -> {:?}; {:?}/{:?}/{:?}; {} hybrid transition(s)",
                time.represent::<Gregorian, Utc>(path.start().instant())?,
                time.represent::<Gregorian, Utc>(path.end().instant())?,
                path.start_character(),
                path.greatest_character(),
                path.end_character(),
                path.transitions().len(),
            );
            let geographic = events.solar_eclipse_path(
                &eclipse,
                &polynomial,
                time.delta_t_at(maximum.instant())?,
                GlobalSolarEclipsePathOptions::standard(),
            )?;
            let greatest = geographic
                .points()
                .iter()
                .find(|point| point.instant() == maximum.instant())
                .expect("greatest instant is retained as a geographic path sample");
            let centre = greatest.centre_line();
            println!(
                "  centre={:+.4}° {:+.4}° north={:+.4}° {:+.4}° south={:+.4}° {:+.4}°",
                centre.latitude().as_degrees(),
                centre.longitude().as_degrees(),
                greatest.northern_limit().latitude().as_degrees(),
                greatest.northern_limit().longitude().as_degrees(),
                greatest.southern_limit().latitude().as_degrees(),
                greatest.southern_limit().longitude().as_degrees(),
            );
            println!(
                "  boundary span={:.1} km path width={:.1} km central duration={:.1} s Sun alt={:.1}° az={:.1}°",
                greatest.boundary_geodesic_span().as_kilometres(),
                greatest.path_width().as_kilometres(),
                greatest.central_duration().as_seconds_f64(),
                greatest.sun_direction().altitude().as_degrees(),
                greatest
                    .sun_direction()
                    .azimuth()
                    .expect("the Sun is not at the zenith on this path sample")
                    .as_degrees(),
            );
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = TimeContext::builtin();
    let data = IersFinals2000A::parse(include_str!("../data/eop/finals2000a-2026-08-09.all"))?;
    let samples = data.try_earth_rotation_samples_in(
        &base,
        ModifiedJulianDate::<Utc>::from_parts(61_040.0, 0.0)?,
        ModifiedJulianDate::<Utc>::from_parts(61_407.0, 0.0)?,
        EarthOrientationAcceptance::IncludePredicted,
    )?;
    let expires = samples[samples.len() - 1]
        .epoch()
        .checked_add(Duration::from_days(1)?)?;
    let rotation =
        EarthRotationTable::new(&samples, "IERS finals2000A snapshot 2026-08-09", expires)?;
    let time = base.with_earth_rotation(rotation);

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
