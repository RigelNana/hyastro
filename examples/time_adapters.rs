use hyastro::time::{
    DateTime, Gregorian, Hifitime, Instant, Jiff, Tai, Tcb, Tcg, Tdb, TimeContext, UnixTimestamp,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let jiff_adapter = Jiff::new();
    let hifitime_adapter = Hifitime::new();

    let jiff_date = jiff::civil::Date::new(2024, 2, 29)?;
    let date = jiff_adapter.import_date(jiff_date)?;
    assert_eq!(jiff_adapter.export_date(date)?, jiff_date);

    let jiff_datetime = jiff::civil::DateTime::new(2026, 8, 6, 12, 34, 56, 789_123_456)?;
    let utc_label = jiff_adapter.import_utc_label(jiff_datetime)?;
    assert_eq!(jiff_adapter.export_utc_label(utc_label)?, jiff_datetime);

    let jiff_timestamp = jiff::Timestamp::from_nanosecond(1_700_000_000_123_456_789)?;
    let unix = jiff_adapter.import_timestamp(jiff_timestamp);
    let utc = hifitime_adapter.resolve_unix(unix);
    assert_eq!(
        jiff_adapter.export_timestamp(hifitime_adapter.unix_timestamp(utc))?,
        jiff_timestamp
    );

    let context = TimeContext::builtin();
    let tai_label =
        DateTime::<Gregorian, Tai>::from_components(2006, 1, 15, 12, 34, 56, 789_123_456)?;
    let tai = context.resolve(tai_label)?;
    let exported_epoch = hifitime_adapter.export(tai);
    assert_eq!(hifitime_adapter.import::<Tai>(exported_epoch), tai);

    let tcg = Instant::<Tcg>::from_instant(tai, &hifitime_adapter)?;
    let tdb = Instant::<Tdb>::from_instant(tai, &hifitime_adapter)?;
    let tcb = Instant::<Tcb>::from_instant(tai, &hifitime_adapter)?;
    let tcg_label = hifitime_adapter.represent::<Gregorian, Tcg>(tcg)?;
    let tdb_label = hifitime_adapter.represent::<Gregorian, Tdb>(tdb)?;
    let tcb_label = hifitime_adapter.represent::<Gregorian, Tcb>(tcb)?;
    assert_eq!(hifitime_adapter.resolve(tcg_label)?, tcg);
    assert_eq!(hifitime_adapter.resolve(tdb_label)?, tdb);
    assert_eq!(hifitime_adapter.resolve(tcb_label)?, tcb);

    let negative_unix = UnixTimestamp::from_nanoseconds(-1_234_567_890_123_456_789);
    assert_eq!(
        hifitime_adapter.unix_timestamp(hifitime_adapter.resolve_unix(negative_unix)),
        negative_unix
    );

    println!("jiff date       = {jiff_date}");
    println!("jiff UTC label  = {jiff_datetime}");
    println!("Unix timestamp  = {} ns", unix.as_nanoseconds());
    println!("hifitime epoch  = {exported_epoch:?}");
    println!("TCG label       = {tcg_label:?}");
    println!("TDB label       = {tdb_label:?}");
    println!("TCB label       = {tcb_label:?}");

    Ok(())
}
