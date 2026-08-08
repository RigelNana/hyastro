use hyastro::{
    math::Length,
    time::{BesselianEpoch, Duration, JulianDate, JulianEpoch, TimeOfDay, Tt, UnixTimestamp},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let astronomical_unit = Length::from_astronomical_units(1.0)?;
    let light_second = Length::from_light_seconds(1.0)?;
    let parsec = Length::from_parsecs(1.0)?;

    assert_eq!(astronomical_unit.as_metres(), Length::METRES_PER_AU);
    assert_eq!(light_second.as_metres(), Length::METRES_PER_LIGHT_SECOND);
    assert_eq!(parsec.as_metres(), Length::METRES_PER_PARSEC);
    assert_eq!(Duration::NANOSECONDS_PER_SECOND, 1_000_000_000);
    assert_eq!(
        Duration::NANOSECONDS_PER_DAY,
        86_400 * Duration::NANOSECONDS_PER_SECOND
    );
    assert_eq!(JulianDate::<Tt>::J2000_VALUE, 2_451_545.0);
    assert_eq!(JulianEpoch::J2000.value(), 2000.0);
    assert_eq!(JulianEpoch::J2016.value(), 2016.0);
    assert_eq!(BesselianEpoch::B1950.value(), 1950.0);
    assert_eq!(TimeOfDay::MIDNIGHT.hour(), 0);
    assert_eq!(UnixTimestamp::EPOCH.as_nanoseconds(), 0);

    println!("1 au       = {:.0} m", astronomical_unit.as_metres());
    println!(
        "1 au       = {:.9} light-seconds",
        astronomical_unit.as_light_seconds()
    );
    println!("1 parsec   = {:.9e} m", parsec.as_metres());
    println!("J2000.0 TT = JD {:.1}", JulianDate::<Tt>::J2000_VALUE);
    println!(
        "Julian year = {} exact nanoseconds",
        Duration::NANOSECONDS_PER_JULIAN_YEAR
    );

    Ok(())
}
