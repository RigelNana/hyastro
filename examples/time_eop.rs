use hyastro::time::{
    DateTime, Duration, EarthOrientationAcceptance, EarthOrientationProduct, EarthOrientationTable,
    GeocentricTdb, Gregorian, IersC04, IersFinals2000A, JulianDate, ModifiedJulianDate, Tdb,
    TimeContext, Ut1, Utc,
};

const C04: &str = include_str!("../data/eop/eop-20u24-c04-1962-now-2026-08-06.txt");
const FINALS_2000_A: &str = include_str!("../data/eop/finals2000a-2026-08-06.all");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let c04 = IersC04::parse(C04)?;
    let finals = IersFinals2000A::parse(FINALS_2000_A)?;
    assert_eq!(c04.product(), EarthOrientationProduct::IersC04);
    assert_eq!(finals.product(), EarthOrientationProduct::IersFinals2000A);

    let base = TimeContext::builtin();
    let samples = c04.try_samples_in(
        &base,
        ModifiedJulianDate::<Utc>::from_parts(54_387.0, 0.0)?,
        ModifiedJulianDate::<Utc>::from_parts(54_390.0, 0.0)?,
        EarthOrientationAcceptance::FinalOnly,
    )?;
    let expires = samples
        .last()
        .expect("the requested C04 interval contains records")
        .epoch()
        .checked_add(Duration::from_days(1)?)?;
    let table = EarthOrientationTable::new(&samples, "IERS C04 example interval", expires)?;
    let time = base.with_earth_orientation(table);
    let epoch = time.resolve(DateTime::<Gregorian, Utc>::from_components(
        2007, 10, 15, 12, 0, 0, 0,
    )?)?;
    let orientation = time.earth_orientation_at(epoch)?;
    let ut1 = JulianDate::<Ut1>::from_instant(epoch, &time)?;
    let delta_t = time.delta_t_at(epoch)?;
    let tdb_model = GeocentricTdb::new();
    let tdb = tdb_model.at(epoch)?;
    let tdb_julian = JulianDate::<Tdb>::from_instant(epoch, &tdb_model)?;
    let (coverage_start, coverage_end) = table.coverage();

    assert!(orientation.polar_motion_rate_x().is_some());
    assert!(orientation.polar_motion_rate_y().is_some());
    assert!(orientation.celestial_pole_offset_rate_x().is_some());
    assert!(orientation.celestial_pole_offset_rate_y().is_some());

    println!(
        "C04 product          = {:?}, records={}",
        c04.product(),
        c04.records().len()
    );
    println!(
        "finals2000A product  = {:?}, records={}",
        finals.product(),
        finals.records().len()
    );
    println!("selected samples      = {}", samples.len());
    println!(
        "coverage TAI ns      = {} .. {}",
        coverage_start.tai_nanoseconds_since_1900(),
        coverage_end.tai_nanoseconds_since_1900()
    );
    println!("UT1 Julian Date       = {:.12}", ut1.as_f64_lossy());
    println!(
        "UT1−UTC              = {:.9} s",
        orientation.ut1_minus_utc().as_seconds()
    );
    println!("Delta T               = {:.9} s", delta_t.as_seconds());
    println!(
        "TDB−TT                = {:+.9} s",
        tdb.tdb_minus_tt_seconds()
    );
    println!("TDB Julian Date       = {:.12}", tdb_julian.as_f64_lossy());
    println!(
        "excess LOD           = {:.6} ms",
        orientation.excess_length_of_day().as_milliseconds()
    );
    println!(
        "polar motion         = ({:.9}, {:.9}) arcsec",
        orientation.polar_motion_x().as_arcseconds(),
        orientation.polar_motion_y().as_arcseconds()
    );
    println!(
        "celestial offsets    = ({:.9}, {:.9}) mas",
        orientation.celestial_pole_offset_x().as_milliarcseconds(),
        orientation.celestial_pole_offset_y().as_milliarcseconds()
    );

    Ok(())
}
