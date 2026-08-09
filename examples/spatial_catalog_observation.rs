use std::{env, error::Error, fs};

use hyastro::{
    astro::{AstrometricSpatialCatalogPlace, Astrometry},
    catalog::{CatalogProperMotion, CatalogRadialVelocity, Parallax, SpatialCatalogPlace},
    earth::{Earth, EllipsoidalHeight, GeodeticLatitude, GeodeticLongitude, GeodeticPosition},
    ephem::{Ephemeris, KernelManifest},
    frame::{EquatorialDirection, Icrs},
    math::{Declination, RightAscension},
    time::{
        DateTime, Duration, EarthOrientationAcceptance, EarthOrientationTable, Gregorian, IersC04,
        JulianDate, ModifiedJulianDate, Tcb, TimeContext, Utc,
    },
};

const RADIANS_PER_MILLIARCSECOND: f64 = core::f64::consts::PI / 648_000_000.0;

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let kernel_path = arguments
        .next()
        .ok_or("missing DE BSP path; usage: spatial_catalog_observation DE_BSP IERS_C04")?;
    let eop_path = arguments
        .next()
        .ok_or("missing IERS C04 path; usage: spatial_catalog_observation DE_BSP IERS_C04")?;
    if arguments.next().is_some() {
        return Err("usage: spatial_catalog_observation DE_BSP IERS_C04".into());
    }

    let base = TimeContext::builtin();
    let eop_text = fs::read_to_string(eop_path)?;
    let product = IersC04::parse(&eop_text)?;
    let samples = product.try_samples_in(
        &base,
        ModifiedJulianDate::<Utc>::from_parts(53_700.0, 0.0)?,
        ModifiedJulianDate::<Utc>::from_parts(53_800.0, 0.0)?,
        EarthOrientationAcceptance::FinalOnly,
    )?;
    let expires = samples[samples.len() - 1]
        .epoch()
        .checked_add(Duration::from_days(1)?)?;
    let time = base.with_earth_orientation(EarthOrientationTable::new(
        &samples,
        "IERS C04 spatial-catalog example",
        expires,
    )?);
    let ephemeris = Ephemeris::load(KernelManifest::inspect([kernel_path])?)?;
    let epoch = base.resolve(DateTime::<Gregorian, Utc>::from_components(
        2006, 1, 1, 0, 0, 0, 0,
    )?)?;
    let site = Earth::wgs84().fixed_site(
        "Greenwich",
        GeodeticPosition::new(
            GeodeticLongitude::try_from_degrees(0.0)?,
            GeodeticLatitude::try_from_degrees(51.4779)?,
            EllipsoidalHeight::from_metres(46.0)?,
        ),
    )?;

    // IAU SOFA starpm/starpv validation source parameters.
    let declination = Declination::try_from_radians(-1.093_989_828)?;
    let catalog = SpatialCatalogPlace::new(
        JulianDate::<Tcb>::from_parts(2_400_000.5, 50_083.0)?,
        EquatorialDirection::<Icrs>::new(RightAscension::wrap_radians(0.016_867_56)?, declination),
        CatalogProperMotion::from_radians_per_julian_year(
            -1.783_235_16e-5 * declination.as_radians().cos(),
            2.336_024_047e-6,
        )?,
        Parallax::from_arcseconds(0.747_23)?,
        CatalogRadialVelocity::from_kilometres_per_second(-21.6)?,
    );

    let astrometry = Astrometry::new(&time, &ephemeris);
    let observer = astrometry.fixed_observer_at(&site, epoch)?;
    let astrometric = AstrometricSpatialCatalogPlace::from_spatial_catalog(catalog, epoch)?;
    let vacuum = observer.vacuum_observed_spatial_catalog_place(astrometric)?;
    let astrometric_coordinates = astrometric.direction().coordinates();
    let azimuth = vacuum
        .horizontal()
        .azimuth()
        .ok_or("azimuth is undefined at the zenith or nadir")?;

    println!("epoch = 2006-01-01T00:00:00 UTC");
    println!(
        "SSB astrometric RA = {:.12} deg, Dec = {:+.12} deg",
        astrometric_coordinates.right_ascension().as_degrees(),
        astrometric_coordinates.declination().as_degrees(),
    );
    println!(
        "observer parallax = {:.6} mas",
        vacuum.corrections().parallax().as_radians() / RADIANS_PER_MILLIARCSECOND,
    );
    println!(
        "vacuum azimuth = {:.9} deg, altitude = {:+.9} deg",
        azimuth.as_degrees(),
        vacuum.horizontal().altitude().as_degrees(),
    );

    Ok(())
}
