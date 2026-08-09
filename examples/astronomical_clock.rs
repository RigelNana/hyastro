//! 每秒刷新一次的固定观测站实时天文钟。
//!
//! 仪表盘展示本库无需区间搜索或外部星表即可计算的瞬时站心量。
//! 按 `1`–`6` 或方向键切换页面，按空格暂停，按 `r` 刷新，按 `q` 退出。

use std::{
    env,
    error::Error,
    io::{Error as IoError, ErrorKind},
    path::PathBuf,
    time::{Duration as StdDuration, Instant as StdInstant},
};

use hyastro::{
    astro::{
        AirTemperature, Astrometry, AtmosphericConditions, AtmosphericPressure, FixedObserverAt,
        GeocentricApparentPlace, HorizonsCompatibleLunarV, LunarVApplicability, MoonPhaseBranch,
        ObservedPlace, ObservingWavelength, ReceptionLightTimeOptions, RefractionAccuracy,
        RelativeHumidity, SolarDeflectionDisposition, VacuumObservedPlace,
    },
    earth::{
        Earth, EllipsoidalHeight, FixedSite, GeodeticLatitude, GeodeticLongitude, GeodeticPosition,
        SiteVelocityModel,
    },
    ephem::{CelestialBody, Ephemeris, KernelManifest, SphericalBodyFigure},
    event::{
        AngularEventSearchOptions, Events, HorizonCriterion, HorizonDiskPoint, HorizonEventKind,
        HorizonSearchOptions, SolarTermEvent,
    },
    frame::{
        CelestialOrientationSolution, EquatorialDirection, Error as FrameError, Frames,
        MeanEquatorEquinoxJ2000,
    },
    math::{Altitude, Longitude},
    time::{
        DateTime, Duration, EarthAttitudeTable, EarthOrientationAcceptance, FixedUtcOffset,
        Gregorian, Hifitime, IersFinals2000A, Instant as AstroInstant, Jiff, JulianDate,
        ModifiedJulianDate, Tai, Tdb, TimeContext, TimeInterval, TimeOfDay, TimeScale, Tt, Ut1,
        Utc,
    },
    uncertainty::UncertaintyOrigin,
};
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Padding, Paragraph, Row, Table, Tabs, Wrap},
};

const FINALS_2000_A: &str = include_str!("../data/eop/finals2000a-2026-08-09.all");
const DEFAULT_KERNEL_PATH: &str = "data/ephem/de440.bsp";
const EOP_VERSION: &str = "IERS finals2000A 快照 2026-08-09";

const LATITUDE_DEGREES: f64 = 31.340_370;
const LONGITUDE_DEGREES: f64 = 121.458_930;
const HEIGHT_METRES: f64 = 15.0;
const PRESSURE_HECTOPASCALS: f64 = 1_013.25;
const TEMPERATURE_CELSIUS: f64 = 15.0;
const RELATIVE_HUMIDITY: f64 = 0.50;
const WAVELENGTH_MICROMETRES: f64 = 0.550;

const BACKGROUND: Color = Color::Rgb(8, 12, 16);
const SURFACE: Color = Color::Rgb(14, 20, 25);
const BORDER: Color = Color::Rgb(54, 68, 75);
const TEXT: Color = Color::Rgb(226, 229, 224);
const MUTED: Color = Color::Rgb(126, 139, 143);
const CYAN: Color = Color::Rgb(112, 184, 174);
const AMBER: Color = Color::Rgb(222, 173, 96);
const MOONLIGHT: Color = Color::Rgb(177, 190, 205);
const GREEN: Color = Color::Rgb(124, 185, 143);
const RED: Color = Color::Rgb(211, 112, 116);

#[derive(Debug)]
struct Cli {
    kernel_path: PathBuf,
    plain: bool,
}

impl Cli {
    const USAGE: &'static str = "用法：cargo run --release --features anise,jiff --example astronomical_clock -- [DE440_BSP路径] [--plain]";

    fn from_process() -> Result<Option<Self>, IoError> {
        let mut kernel_path = None;
        let mut plain = false;
        for argument in env::args_os().skip(1) {
            match argument.to_str() {
                Some("--help" | "-h") => {
                    println!("{}", Self::USAGE);
                    println!("默认 DE440 BSP：{DEFAULT_KERNEL_PATH}");
                    println!("--plain 仅打印一次当前快照，不进入终端界面");
                    return Ok(None);
                }
                Some("--plain") => plain = true,
                _ if kernel_path.is_none() => kernel_path = Some(PathBuf::from(argument)),
                _ => return Err(IoError::new(ErrorKind::InvalidInput, Self::USAGE)),
            }
        }
        Ok(Some(Self {
            kernel_path: kernel_path.unwrap_or_else(|| PathBuf::from(DEFAULT_KERNEL_PATH)),
            plain,
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Overview,
    Sun,
    Moon,
    Bodies,
    Earth,
    Observer,
    SolarTerms,
    Uncertainty,
}

impl Page {
    const ALL: [Self; 8] = [
        Self::Overview,
        Self::Sun,
        Self::Moon,
        Self::Bodies,
        Self::Earth,
        Self::Observer,
        Self::SolarTerms,
        Self::Uncertainty,
    ];

    const fn index(self) -> usize {
        match self {
            Self::Overview => 0,
            Self::Sun => 1,
            Self::Moon => 2,
            Self::Bodies => 3,
            Self::Earth => 4,
            Self::Observer => 5,
            Self::SolarTerms => 6,
            Self::Uncertainty => 7,
        }
    }

    const fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    const fn previous(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

struct App {
    snapshot: Snapshot,
    page: Page,
    paused: bool,
    last_error: Option<String>,
}

impl App {
    fn new(snapshot: Snapshot) -> Self {
        Self {
            snapshot,
            page: Page::Overview,
            paused: false,
            last_error: None,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press {
            return false;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char(' ') => self.paused = !self.paused,
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => self.page = self.page.next(),
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                self.page = self.page.previous();
            }
            KeyCode::Char('1') => self.page = Page::Overview,
            KeyCode::Char('2') => self.page = Page::Sun,
            KeyCode::Char('3') => self.page = Page::Moon,
            KeyCode::Char('4') => self.page = Page::Bodies,
            KeyCode::Char('5') => self.page = Page::Earth,
            KeyCode::Char('6') => self.page = Page::Observer,
            KeyCode::Char('7') => self.page = Page::SolarTerms,
            KeyCode::Char('8') => self.page = Page::Uncertainty,
            _ => {}
        }
        false
    }
}

struct Snapshot {
    clocks: ClockSnapshot,
    earth: EarthSnapshot,
    observer: ObserverSnapshot,
    sun: SunSnapshot,
    moon: MoonSnapshot,
    bodies: Vec<BodySnapshot>,
    daily_events: DailyEvents,
    solar_term: SolarTermSnapshot,
    solar_term_year: SolarTermYearSnapshot,
    data: DataSnapshot,
}

struct ClockSnapshot {
    utc: String,
    tai: String,
    tt: String,
    ut1: String,
    tdb: String,
    mean_solar: String,
    apparent_solar: String,
    equation_of_time_minutes: f64,
    era_hours: f64,
    gmst_hours: f64,
    gast_hours: f64,
    lmst_hours: f64,
    last_hours: f64,
    jd_utc: f64,
    mjd_utc: f64,
    jd_tt: f64,
    jd_ut1: f64,
    jd_tdb: f64,
}

struct EarthSnapshot {
    ut1_minus_utc_seconds: f64,
    ut1_minus_utc_standard_uncertainty_seconds: Option<f64>,
    delta_t_seconds: f64,
    polar_motion_x_arcseconds: f64,
    polar_motion_y_arcseconds: f64,
    polar_motion_x_standard_uncertainty_arcseconds: Option<f64>,
    polar_motion_y_standard_uncertainty_arcseconds: Option<f64>,
    pole_offset_x_milliarcseconds: f64,
    pole_offset_y_milliarcseconds: f64,
    pole_offset_x_standard_uncertainty_milliarcseconds: Option<f64>,
    pole_offset_y_standard_uncertainty_milliarcseconds: Option<f64>,
    eop_standard_uncertainty_origin: Option<UncertaintyOrigin>,
    cip_x_arcseconds: f64,
    cip_y_arcseconds: f64,
    tio_locator_milliarcseconds: f64,
    mean_obliquity_degrees: f64,
    true_obliquity_degrees: f64,
    nutation_longitude_arcseconds: f64,
    nutation_obliquity_arcseconds: f64,
    equation_of_origins_arcseconds: f64,
    equation_of_equinoxes_arcseconds: f64,
}

struct ObserverSnapshot {
    itrs_kilometres: [f64; 3],
    gcrs_kilometres: [f64; 3],
    gcrs_velocity_metres_per_second: [f64; 3],
    inertial_speed_metres_per_second: f64,
    bcrs_astronomical_units: [f64; 3],
    bcrs_velocity_kilometres_per_second: [f64; 3],
    barycentric_speed_kilometres_per_second: f64,
    velocity_model: &'static str,
}

#[derive(Clone)]
struct SkyObjectSnapshot {
    cirs_right_ascension_hours: f64,
    cirs_declination_degrees: f64,
    hour_angle_hours: f64,
    azimuth_degrees: Option<f64>,
    vacuum_altitude_degrees: f64,
    observed_altitude_degrees: f64,
    zenith_distance_degrees: f64,
    distance_astronomical_units: f64,
    light_time_seconds: f64,
    light_time_iterations: u32,
    light_time_residual_nanoseconds: i128,
    refraction_arcseconds: f64,
    refraction_accuracy: &'static str,
}

#[derive(Clone, Copy)]
struct ApparentCoordinatesSnapshot {
    gcrs_right_ascension_hours: f64,
    gcrs_declination_degrees: f64,
    j2000_right_ascension_hours: f64,
    j2000_declination_degrees: f64,
    mean_right_ascension_hours: f64,
    mean_declination_degrees: f64,
    true_right_ascension_hours: f64,
    true_declination_degrees: f64,
    mean_ecliptic_longitude_degrees: f64,
    mean_ecliptic_latitude_degrees: f64,
    true_ecliptic_longitude_degrees: f64,
    true_ecliptic_latitude_degrees: f64,
}

struct SunSnapshot {
    sky: SkyObjectSnapshot,
    coordinates: ApparentCoordinatesSnapshot,
    angular_diameter_arcminutes: f64,
    deflection_arcseconds: f64,
    deflection_disposition: &'static str,
}

struct MoonSnapshot {
    sky: SkyObjectSnapshot,
    coordinates: ApparentCoordinatesSnapshot,
    angular_diameter_arcminutes: f64,
    branch: &'static str,
    directed_elongation_degrees: f64,
    apparent_separation_degrees: f64,
    phase_angle_degrees: f64,
    illuminated_percent: f64,
    visual_magnitude: f64,
    magnitude_applicability: &'static str,
    magnitude_distance_correction: f64,
    magnitude_phase_correction: f64,
    flux_ratio_to_zero_magnitude: f64,
    sunlight_at_moon_seconds: f64,
}

struct BodySnapshot {
    name: &'static str,
    sky: Option<SkyObjectSnapshot>,
    error: Option<String>,
}

struct DataSnapshot {
    eop_version: String,
    eop_coverage: String,
    kernel_name: String,
    kernel_count: usize,
    calculation_milliseconds: f64,
}

#[derive(Clone)]
struct DailyEvents {
    local_date: String,
    sun: BodyHorizonTimes,
    moon: BodyHorizonTimes,
    solar_terms: Vec<SolarTermEvent<Utc>>,
}

#[derive(Clone)]
struct SolarTermSnapshot {
    current_name: &'static str,
    current_longitude_degrees: f64,
    current_local_time: String,
    current_time_uncertainty_seconds: f64,
    current_residual_arcseconds: f64,
    next_name: &'static str,
    next_longitude_degrees: f64,
    next_local_time: String,
    next_time_uncertainty_seconds: f64,
    next_residual_arcseconds: f64,
    until_next_days: f64,
    until_next_uncertainty_days: f64,
}

#[derive(Clone)]
struct SolarTermYearSnapshot {
    year: i32,
    entries: Vec<SolarTermEntrySnapshot>,
}

#[derive(Clone)]
struct SolarTermEntrySnapshot {
    name: &'static str,
    longitude_degrees: f64,
    local_time: String,
    compact_local_time: String,
    uncertainty_seconds: f64,
    residual_arcseconds: f64,
}

struct CriterionTimes {
    rise: Option<String>,
    upper_transit: Option<String>,
    set: Option<String>,
}
#[derive(Clone)]
struct BodyHorizonTimes {
    vacuum: HorizonTimes,
    standard_refracted: HorizonTimes,
}

#[derive(Clone)]
struct HorizonTimes {
    upper_limb_rise: Option<String>,
    rise: Option<String>,
    lower_limb_rise: Option<String>,
    upper_transit: Option<String>,
    lower_limb_set: Option<String>,
    set: Option<String>,
    upper_limb_set: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let Some(cli) = Cli::from_process()? else {
        return Ok(());
    };

    let base = TimeContext::builtin();
    let startup_epoch = current_epoch();
    let current_mjd = JulianDate::<Utc>::from_instant(startup_epoch, &base)?
        .to_modified()?
        .as_f64_lossy()
        .floor();
    let eop_data = IersFinals2000A::parse(FINALS_2000_A)?;
    let attitude_samples = eop_data.try_earth_attitude_samples_in(
        &base,
        ModifiedJulianDate::<Utc>::from_parts(current_mjd - 1.0, 0.0)?,
        ModifiedJulianDate::<Utc>::from_parts(current_mjd + 2.0, 0.0)?,
        EarthOrientationAcceptance::IncludePredicted,
    )?;
    let expires = attitude_samples
        .last()
        .ok_or_else(|| IoError::new(ErrorKind::InvalidData, "当前 EOP 时间窗口为空"))?
        .epoch()
        .checked_add(Duration::from_days(1)?)?;
    let attitude_table = EarthAttitudeTable::new(&attitude_samples, EOP_VERSION, expires)?;
    let time = base.with_earth_attitude(attitude_table);

    let earth = Earth::wgs84();
    let site = earth.fixed_site(
        "上海实时天文钟",
        GeodeticPosition::new(
            GeodeticLongitude::try_from_degrees(LONGITUDE_DEGREES)?,
            GeodeticLatitude::try_from_degrees(LATITUDE_DEGREES)?,
            EllipsoidalHeight::from_metres(HEIGHT_METRES)?,
        ),
    )?;
    let atmosphere = AtmosphericConditions::new(
        AtmosphericPressure::from_hectopascals(PRESSURE_HECTOPASCALS)?,
        AirTemperature::from_degrees_celsius(TEMPERATURE_CELSIUS)?,
        RelativeHumidity::from_fraction(RELATIVE_HUMIDITY)?,
        ObservingWavelength::from_micrometres(WAVELENGTH_MICROMETRES)?,
    );
    let ephemeris = Ephemeris::load(KernelManifest::inspect([cli.kernel_path])?)?;
    let snapshot = calculate_snapshot(&time, &ephemeris, &site, atmosphere, None, None)?;

    if cli.plain {
        print_plain(&snapshot);
        return Ok(());
    }

    ratatui::run(|terminal| run_clock(terminal, &time, &ephemeris, &site, atmosphere, snapshot))?;
    Ok(())
}

fn current_epoch() -> AstroInstant<Utc> {
    Hifitime::new().resolve_unix(Jiff::new().import_timestamp(jiff::Timestamp::now()))
}

fn run_clock(
    terminal: &mut DefaultTerminal,
    time: &TimeContext<'_, EarthAttitudeTable<'_>>,
    ephemeris: &Ephemeris,
    site: &FixedSite,
    atmosphere: AtmosphericConditions,
    initial_snapshot: Snapshot,
) -> std::io::Result<()> {
    let mut app = App::new(initial_snapshot);
    let tick = StdDuration::from_secs(1);
    let mut next_tick = StdInstant::now() + tick;

    loop {
        terminal.draw(|frame| render(frame, &app))?;
        let timeout = if app.paused {
            StdDuration::from_millis(250)
        } else {
            next_tick.saturating_duration_since(StdInstant::now())
        };

        if event::poll(timeout)?
            && let Event::Key(key) = event::read()?
        {
            if app.handle_key(key) {
                return Ok(());
            }
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('r') {
                refresh(&mut app, time, ephemeris, site, atmosphere);
                next_tick = StdInstant::now() + tick;
            }
        }

        let now = StdInstant::now();
        if !app.paused && now >= next_tick {
            refresh(&mut app, time, ephemeris, site, atmosphere);
            while next_tick <= now {
                next_tick += tick;
            }
        }
    }
}

fn refresh(
    app: &mut App,
    time: &TimeContext<'_, EarthAttitudeTable<'_>>,
    ephemeris: &Ephemeris,
    site: &FixedSite,
    atmosphere: AtmosphericConditions,
) {
    match calculate_snapshot(
        time,
        ephemeris,
        site,
        atmosphere,
        Some(&app.snapshot.daily_events),
        Some(&app.snapshot.solar_term_year),
    ) {
        Ok(snapshot) => {
            app.snapshot = snapshot;
            app.last_error = None;
        }
        Err(error) => app.last_error = Some(error.to_string()),
    }
}

fn calculate_snapshot(
    time: &TimeContext<'_, EarthAttitudeTable<'_>>,
    ephemeris: &Ephemeris,
    site: &FixedSite,
    atmosphere: AtmosphericConditions,
    cached_daily_events: Option<&DailyEvents>,
    cached_solar_term_year: Option<&SolarTermYearSnapshot>,
) -> Result<Snapshot, Box<dyn Error>> {
    let started = StdInstant::now();
    let now = current_epoch();
    let hifitime = Hifitime::new();
    let options = ReceptionLightTimeOptions::standard();
    let astrometry = Astrometry::new(time, ephemeris);
    let frames = Frames::new(time);
    let attitude = time.earth_attitude_at(now)?;

    let utc_label = time.represent::<Gregorian, Utc>(now)?;
    let local_offset = Duration::from_seconds(8 * 3_600)?;
    let local_label = time.represent::<Gregorian, Utc>(now.checked_add(local_offset)?)?;
    let local_date = format_date(local_label);
    let daily_events = match cached_daily_events {
        Some(cached) if cached.local_date == local_date => cached.clone(),
        _ => calculate_daily_events(time, ephemeris, site, local_label, local_date, local_offset)?,
    };
    let solar_term = solar_term_snapshot(time, now, local_offset, &daily_events.solar_terms)?;
    let local_year = local_label.date().year();
    let solar_term_year = match cached_solar_term_year {
        Some(cached) if cached.year == local_year => cached.clone(),
        _ => calculate_solar_term_year(time, ephemeris, local_year)?,
    };
    let tai_epoch = AstroInstant::<Tai>::from_instant(now, time)?;
    let tt_epoch = AstroInstant::<Tt>::from_instant(now, time)?;
    let tdb_epoch = AstroInstant::<Tdb>::from_instant(now, &hifitime)?;
    let tai_label = time.represent::<Gregorian, Tai>(tai_epoch)?;
    let tt_label = time.represent::<Gregorian, Tt>(tt_epoch)?;
    let jd_ut1 = JulianDate::<Ut1>::from_instant(now, time)?;
    let ut1_label =
        time.represent::<Gregorian, Utc>(now.checked_add(attitude.ut1_minus_utc().as_duration())?)?;
    let tdb_label = hifitime.represent::<Gregorian, Tdb>(tdb_epoch)?;

    let jd_utc = JulianDate::<Utc>::from_instant(now, time)?;
    let jd_tt = JulianDate::<Tt>::from_instant(now, time)?;
    let jd_tdb = JulianDate::<Tdb>::from_instant(now, &hifitime)?;

    let orientation = frames.earth_attitude_at(now)?;
    let precession_nutation = orientation.precession_nutation();
    let celestial_orientation = frames.celestial_orientation_at(now)?;
    let sidereal = frames.sidereal_time_at(now)?;
    let longitude = Longitude::try_from_radians(site.geodetic_position().longitude().as_radians())?;
    let local_mean_sidereal = sidereal.local_mean_sidereal_time(longitude)?;
    let local_apparent_sidereal = sidereal.local_apparent_sidereal_time(longitude)?;
    let solar_time = astrometry.solar_time(now, options)?;
    let local_solar_time = solar_time.at_longitude(longitude)?;
    let delta_t = time.delta_t_at(now)?;

    let observer = astrometry.fixed_observer_with_nominal_rotation_at(site, now)?;
    let sun_vacuum = observer.vacuum_observed_place(CelestialBody::Sun, options)?;
    let sun_observed = sun_vacuum.apply_refraction(atmosphere)?;
    let sun_disk = sun_vacuum.apparent_disk(SphericalBodyFigure::IAU_2015_NOMINAL_SUN)?;
    let sun_sky = sky_snapshot(sun_vacuum, sun_observed, local_apparent_sidereal.as_hours());
    let apparent_sun = solar_time.apparent_sun();
    let sun_coordinates = apparent_coordinates(apparent_sun.geocentric(), celestial_orientation)?;
    let solar_deflection = sun_vacuum.solar_light_deflection();

    let moon_vacuum = observer.vacuum_observed_place(CelestialBody::Moon, options)?;
    let moon_observed = moon_vacuum.apply_refraction(atmosphere)?;
    let moon_disk = moon_vacuum.apparent_disk(SphericalBodyFigure::IAU_WGCCRE_2015_MOON)?;
    let moon_sky = sky_snapshot(
        moon_vacuum,
        moon_observed,
        local_apparent_sidereal.as_hours(),
    );
    let illumination = astrometry.lunar_illumination_at(now, options)?;
    let moon_magnitude = HorizonsCompatibleLunarV::evaluate(illumination)?;
    let apparent_moon = illumination.apparent_moon();
    let moon_coordinates = apparent_coordinates(apparent_moon, celestial_orientation)?;

    let mut bodies = Vec::with_capacity(10);
    bodies.push(BodySnapshot {
        name: "太阳",
        sky: Some(sun_sky.clone()),
        error: None,
    });
    bodies.push(BodySnapshot {
        name: "月球",
        sky: Some(moon_sky.clone()),
        error: None,
    });
    for (name, target) in [
        ("水星", CelestialBody::Mercury),
        ("金星", CelestialBody::Venus),
        ("火星系统", CelestialBody::MarsBarycenter),
        ("木星系统", CelestialBody::JupiterBarycenter),
        ("土星系统", CelestialBody::SaturnBarycenter),
        ("天王星系统", CelestialBody::UranusBarycenter),
        ("海王星系统", CelestialBody::NeptuneBarycenter),
        ("冥王星系统", CelestialBody::PlutoBarycenter),
    ] {
        bodies.push(calculate_body(
            observer,
            atmosphere,
            options,
            local_apparent_sidereal.as_hours(),
            name,
            target,
        ));
    }

    let topocentric = observer.topocentric_frame();
    let observer_state = topocentric.observer_state();
    let itrs_components = site.itrs_position().position().components();
    let gcrs_components = observer_state.position().position().components();
    let velocity_components = observer_state.velocity().components();
    let barycentric_position = observer.barycentric_position();
    let barycentric_velocity = observer.barycentric_velocity();
    let barycentric_position_components = barycentric_position.components();
    let barycentric_velocity_components = barycentric_velocity.components();
    let coverage = time.earth_attitude().coverage();
    let coverage_start = time.represent::<Gregorian, Utc>(coverage.0)?;
    let coverage_end = time.represent::<Gregorian, Utc>(coverage.1)?;
    let kernel_name = ephemeris
        .manifest()
        .kernels()
        .first()
        .and_then(|kernel| kernel.path().file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "未命名内核".to_owned());

    let clocks = ClockSnapshot {
        utc: format_datetime(utc_label),
        tai: format_datetime(tai_label),
        tt: format_datetime(tt_label),
        ut1: format_datetime(ut1_label),
        tdb: format_datetime(tdb_label),
        mean_solar: format_time_of_day(local_solar_time.mean_solar_time().as_time_of_day()),
        apparent_solar: format_time_of_day(local_solar_time.apparent_solar_time().as_time_of_day()),
        equation_of_time_minutes: local_solar_time.equation_of_time().as_minutes(),
        era_hours: sidereal.earth_rotation_angle().as_hours(),
        gmst_hours: sidereal.greenwich_mean_sidereal_time().as_hours(),
        gast_hours: sidereal.greenwich_apparent_sidereal_time().as_hours(),
        lmst_hours: local_mean_sidereal.as_hours(),
        last_hours: local_apparent_sidereal.as_hours(),
        jd_utc: jd_utc.as_f64_lossy(),
        mjd_utc: jd_utc.to_modified()?.as_f64_lossy(),
        jd_tt: jd_tt.as_f64_lossy(),
        jd_ut1: jd_ut1.as_f64_lossy(),
        jd_tdb: jd_tdb.as_f64_lossy(),
    };
    let earth_standard_uncertainties = attitude.standard_uncertainties();
    let earth = EarthSnapshot {
        ut1_minus_utc_seconds: attitude.ut1_minus_utc().as_seconds(),
        ut1_minus_utc_standard_uncertainty_seconds: earth_standard_uncertainties
            .ut1_minus_utc()
            .map(|value| value.value().as_seconds_f64()),
        delta_t_seconds: delta_t.as_seconds(),
        polar_motion_x_arcseconds: attitude.polar_motion_x().as_arcseconds(),
        polar_motion_y_arcseconds: attitude.polar_motion_y().as_arcseconds(),
        polar_motion_x_standard_uncertainty_arcseconds: earth_standard_uncertainties
            .polar_motion_x()
            .map(|value| value.value().as_degrees() * 3_600.0),
        polar_motion_y_standard_uncertainty_arcseconds: earth_standard_uncertainties
            .polar_motion_y()
            .map(|value| value.value().as_degrees() * 3_600.0),
        pole_offset_x_milliarcseconds: attitude.celestial_pole_offset_x().as_milliarcseconds(),
        pole_offset_y_milliarcseconds: attitude.celestial_pole_offset_y().as_milliarcseconds(),
        pole_offset_x_standard_uncertainty_milliarcseconds: earth_standard_uncertainties
            .celestial_pole_offset_x()
            .map(|value| value.value().as_degrees() * 3_600_000.0),
        pole_offset_y_standard_uncertainty_milliarcseconds: earth_standard_uncertainties
            .celestial_pole_offset_y()
            .map(|value| value.value().as_degrees() * 3_600_000.0),
        eop_standard_uncertainty_origin: attitude.standard_uncertainty_origin(),
        cip_x_arcseconds: orientation.cip().x().as_degrees() * 3_600.0,
        cip_y_arcseconds: orientation.cip().y().as_degrees() * 3_600.0,
        tio_locator_milliarcseconds: orientation.tio_locator().as_degrees() * 3_600_000.0,
        mean_obliquity_degrees: precession_nutation.mean_obliquity().as_degrees(),
        true_obliquity_degrees: precession_nutation.true_obliquity().as_degrees(),
        nutation_longitude_arcseconds: precession_nutation.nutation_longitude().as_degrees()
            * 3_600.0,
        nutation_obliquity_arcseconds: precession_nutation.nutation_obliquity().as_degrees()
            * 3_600.0,
        equation_of_origins_arcseconds: sidereal.equation_of_origins().as_degrees() * 3_600.0,
        equation_of_equinoxes_arcseconds: sidereal.equation_of_equinoxes().as_degrees() * 3_600.0,
    };
    let observer = ObserverSnapshot {
        itrs_kilometres: [
            itrs_components[0].as_kilometres(),
            itrs_components[1].as_kilometres(),
            itrs_components[2].as_kilometres(),
        ],
        gcrs_kilometres: [
            gcrs_components[0].as_kilometres(),
            gcrs_components[1].as_kilometres(),
            gcrs_components[2].as_kilometres(),
        ],
        gcrs_velocity_metres_per_second: [
            velocity_components[0].as_metres_per_second(),
            velocity_components[1].as_metres_per_second(),
            velocity_components[2].as_metres_per_second(),
        ],
        inertial_speed_metres_per_second: observer_state
            .velocity()
            .magnitude()?
            .as_metres_per_second(),
        bcrs_astronomical_units: [
            barycentric_position_components[0].as_astronomical_units(),
            barycentric_position_components[1].as_astronomical_units(),
            barycentric_position_components[2].as_astronomical_units(),
        ],
        bcrs_velocity_kilometres_per_second: [
            barycentric_velocity_components[0].as_kilometres_per_second(),
            barycentric_velocity_components[1].as_kilometres_per_second(),
            barycentric_velocity_components[2].as_kilometres_per_second(),
        ],
        barycentric_speed_kilometres_per_second: barycentric_velocity
            .magnitude()?
            .as_kilometres_per_second(),
        velocity_model: site_velocity_model_label(topocentric.velocity_model()),
    };
    let sun = SunSnapshot {
        sky: sun_sky,
        coordinates: sun_coordinates,
        angular_diameter_arcminutes: sun_disk.diameter().as_degrees() * 60.0,
        deflection_arcseconds: solar_deflection.correction().as_degrees() * 3_600.0,
        deflection_disposition: solar_deflection_label(solar_deflection.disposition()),
    };
    let moon = MoonSnapshot {
        sky: moon_sky,
        coordinates: moon_coordinates,
        angular_diameter_arcminutes: moon_disk.diameter().as_degrees() * 60.0,
        branch: moon_phase_branch_label(illumination.branch()),
        directed_elongation_degrees: illumination.directed_elongation().as_degrees(),
        apparent_separation_degrees: illumination.apparent_separation().as_degrees(),
        phase_angle_degrees: illumination.phase_angle().as_degrees(),
        illuminated_percent: illumination.illuminated_fraction().as_percent(),
        visual_magnitude: moon_magnitude.magnitude().as_magnitudes(),
        magnitude_applicability: lunar_v_applicability_label(moon_magnitude.applicability()),
        magnitude_distance_correction: moon_magnitude.distance_correction().as_magnitudes(),
        magnitude_phase_correction: moon_magnitude.phase_correction().as_magnitudes(),
        flux_ratio_to_zero_magnitude: moon_magnitude.flux_ratio_to_zero_magnitude()?.as_ratio(),
        sunlight_at_moon_seconds: illumination
            .sunlight_at_moon()
            .light_time()
            .as_seconds_f64(),
    };
    let data = DataSnapshot {
        eop_version: time.earth_attitude().version().to_owned(),
        eop_coverage: format!(
            "{} — {} UTC",
            format_date(coverage_start),
            format_date(coverage_end)
        ),
        kernel_name,
        kernel_count: ephemeris.manifest().kernel_count(),
        calculation_milliseconds: started.elapsed().as_secs_f64() * 1_000.0,
    };

    Ok(Snapshot {
        clocks,
        earth,
        observer,
        sun,
        moon,
        bodies,
        daily_events,
        solar_term,
        solar_term_year,
        data,
    })
}

fn calculate_daily_events(
    time: &TimeContext<'_, EarthAttitudeTable<'_>>,
    ephemeris: &Ephemeris,
    site: &FixedSite,
    local_label: DateTime<Gregorian, Utc>,
    local_date: String,
    local_offset: Duration,
) -> Result<DailyEvents, Box<dyn Error>> {
    let date = local_label.date();
    let local_midnight_as_utc = DateTime::<Gregorian, Utc>::from_components(
        date.year(),
        date.month(),
        date.day(),
        0,
        0,
        0,
        0,
    )?;
    let start = time
        .resolve(local_midnight_as_utc)?
        .checked_sub(local_offset)?;
    let end = start.checked_add(Duration::from_days(1)?)?;
    let interval = TimeInterval::new(start, end)?;
    let events = Events::new(Astrometry::new(time, ephemeris));
    let options = HorizonSearchOptions::standard();
    let standard_refracted_altitude = Altitude::try_from_degrees(-34.0 / 60.0)?;

    let solve_criterion = |target: CelestialBody,
                           criterion: HorizonCriterion|
     -> Result<CriterionTimes, Box<dyn Error>> {
        let search = events
            .horizon_events_with_nominal_rotation_in(site, target, interval, criterion, options)?;
        let mut rise = None;
        let mut upper_transit = None;
        let mut set = None;
        for event in search.events() {
            let label =
                time.represent::<Gregorian, Utc>(event.instant().checked_add(local_offset)?)?;
            let evidence = event.evidence();
            let value = format!(
                "{} · ±{:.6}s · |Δf|={:.3e}″",
                format_time_of_day(label.time()),
                evidence.time_uncertainty().as_seconds_f64(),
                (evidence.residual().as_degrees() * 3_600.0).abs(),
            );
            match event.kind() {
                HorizonEventKind::Rise => rise = Some(value),
                HorizonEventKind::UpperTransit => upper_transit = Some(value),
                HorizonEventKind::Set => set = Some(value),
                HorizonEventKind::LowerTransit => {}
                _ => {}
            }
        }
        Ok(CriterionTimes {
            rise,
            upper_transit,
            set,
        })
    };

    let solve_reference = |target: CelestialBody,
                           center_criterion: HorizonCriterion,
                           upper_limb_criterion: HorizonCriterion,
                           lower_limb_criterion: HorizonCriterion|
     -> Result<HorizonTimes, Box<dyn Error>> {
        let center = solve_criterion(target, center_criterion)?;
        let upper_limb = solve_criterion(target, upper_limb_criterion)?;
        let lower_limb = solve_criterion(target, lower_limb_criterion)?;
        Ok(HorizonTimes {
            upper_limb_rise: upper_limb.rise,
            rise: center.rise,
            lower_limb_rise: lower_limb.rise,
            upper_transit: center.upper_transit,
            lower_limb_set: lower_limb.set,
            set: center.set,
            upper_limb_set: upper_limb.set,
        })
    };
    let solve = |target: CelestialBody,
                 figure: SphericalBodyFigure|
     -> Result<BodyHorizonTimes, Box<dyn Error>> {
        let vacuum = solve_reference(
            target,
            HorizonCriterion::geometric_center(),
            HorizonCriterion::geometric_upper_limb(figure),
            HorizonCriterion::geometric_lower_limb(figure),
        )?;
        let standard_refracted = solve_reference(
            target,
            HorizonCriterion::vacuum_altitude(standard_refracted_altitude),
            HorizonCriterion::vacuum_disk_altitude(
                standard_refracted_altitude,
                HorizonDiskPoint::UpperLimb(figure),
            ),
            HorizonCriterion::vacuum_disk_altitude(
                standard_refracted_altitude,
                HorizonDiskPoint::LowerLimb(figure),
            ),
        )?;
        Ok(BodyHorizonTimes {
            vacuum,
            standard_refracted,
        })
    };

    let solar_term_interval = TimeInterval::new(
        start.checked_sub(Duration::from_days(32)?)?,
        end.checked_add(Duration::from_days(32)?)?,
    )?;
    let solar_terms =
        events.solar_terms_in(solar_term_interval, AngularEventSearchOptions::standard())?;

    Ok(DailyEvents {
        local_date,
        sun: solve(
            CelestialBody::Sun,
            SphericalBodyFigure::IAU_2015_NOMINAL_SUN,
        )?,
        moon: solve(
            CelestialBody::Moon,
            SphericalBodyFigure::IAU_WGCCRE_2015_MOON,
        )?,
        solar_terms,
    })
}

fn solar_term_snapshot(
    time: &TimeContext<'_, EarthAttitudeTable<'_>>,
    now: AstroInstant<Utc>,
    local_offset: Duration,
    events: &[SolarTermEvent<Utc>],
) -> Result<SolarTermSnapshot, Box<dyn Error>> {
    let current = events
        .iter()
        .rev()
        .find(|event| event.instant() <= now)
        .copied()
        .ok_or_else(|| IoError::new(ErrorKind::InvalidData, "定气窗口缺少当前节气"))?;
    let next = events
        .iter()
        .find(|event| event.instant() > now)
        .copied()
        .ok_or_else(|| IoError::new(ErrorKind::InvalidData, "定气窗口缺少下一节气"))?;
    let local_time = |event: SolarTermEvent<Utc>| -> Result<String, Box<dyn Error>> {
        let label = time.represent::<Gregorian, Utc>(event.instant().checked_add(local_offset)?)?;
        Ok(format!(
            "{} {}",
            format_date(label),
            format_time_of_day(label.time())
        ))
    };
    let current_evidence = current.evidence();
    let next_evidence = next.evidence();

    Ok(SolarTermSnapshot {
        current_name: current.term().chinese_name(),
        current_longitude_degrees: current.term().target_longitude().as_degrees(),
        current_local_time: local_time(current)?,
        current_time_uncertainty_seconds: current_evidence.time_uncertainty().as_seconds_f64(),
        current_residual_arcseconds: (current_evidence.residual().as_degrees() * 3_600.0).abs(),
        next_name: next.term().chinese_name(),
        next_longitude_degrees: next.term().target_longitude().as_degrees(),
        next_local_time: local_time(next)?,
        next_time_uncertainty_seconds: next_evidence.time_uncertainty().as_seconds_f64(),
        next_residual_arcseconds: (next_evidence.residual().as_degrees() * 3_600.0).abs(),
        until_next_days: next.instant().duration_since(now)?.as_seconds_f64() / 86_400.0,
        until_next_uncertainty_days: next_evidence.time_uncertainty().as_seconds_f64() / 86_400.0,
    })
}

fn calculate_solar_term_year(
    time: &TimeContext<'_, EarthAttitudeTable<'_>>,
    ephemeris: &Ephemeris,
    year: i32,
) -> Result<SolarTermYearSnapshot, Box<dyn Error>> {
    let offset = FixedUtcOffset::from_seconds(8 * 3_600)?;
    let computed = Events::new(Astrometry::new(time, ephemeris)).solar_term_year(
        year,
        offset,
        AngularEventSearchOptions::standard(),
    )?;
    let mut entries = Vec::with_capacity(24);
    for entry in computed.entries() {
        let event = entry.event();
        let term = event.term();
        let local = entry.local_time();
        let date = local.date();
        let clock = local.time();
        let evidence = event.evidence();
        entries.push(SolarTermEntrySnapshot {
            name: term.chinese_name(),
            longitude_degrees: term.target_longitude().as_degrees(),
            local_time: format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
                date.year(),
                date.month(),
                date.day(),
                clock.hour(),
                clock.minute(),
                clock.second(),
                clock.nanosecond() / 1_000_000,
            ),
            compact_local_time: format!(
                "{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
                date.month(),
                date.day(),
                clock.hour(),
                clock.minute(),
                clock.second(),
                clock.nanosecond() / 1_000_000,
            ),
            uncertainty_seconds: evidence.time_uncertainty().as_seconds_f64(),
            residual_arcseconds: (evidence.residual().as_degrees() * 3_600.0).abs(),
        });
    }
    Ok(SolarTermYearSnapshot { year, entries })
}

const fn site_velocity_model_label(model: SiteVelocityModel) -> &'static str {
    match model {
        SiteVelocityModel::EarthOrientation => "完整 EOP 地球自转",
        SiteVelocityModel::IersNominalEarthRotation => "IERS 名义地球自转",
        _ => "其他地球自转模型",
    }
}

const fn refraction_accuracy_label(accuracy: RefractionAccuracy) -> &'static str {
    match accuracy {
        RefractionAccuracy::Nominal => "标称精度",
        RefractionAccuracy::HighZenithDistance => "大天顶距",
        RefractionAccuracy::NearHorizon => "近地平线",
        RefractionAccuracy::BelowHorizon => "地平线下（超出验证范围）",
        _ => "其他精度等级",
    }
}

const fn solar_deflection_label(disposition: SolarDeflectionDisposition) -> &'static str {
    match disposition {
        SolarDeflectionDisposition::Applied => "已应用",
        SolarDeflectionDisposition::AppliedToForegroundTarget => "已应用（前景目标）",
        SolarDeflectionDisposition::NotAppliedToSun => "太阳目标不适用",
        SolarDeflectionDisposition::NotAppliedToOccultedTarget => "目标被太阳遮掩",
        _ => "其他处理状态",
    }
}

const fn moon_phase_branch_label(branch: MoonPhaseBranch) -> &'static str {
    match branch {
        MoonPhaseBranch::Waxing => "盈",
        MoonPhaseBranch::Waning => "亏",
    }
}

const fn lunar_v_applicability_label(applicability: LunarVApplicability) -> &'static str {
    match applicability {
        LunarVApplicability::Nominal => "标称适用",
        LunarVApplicability::NearFullMoonKnownBias => "近满月已知偏差",
        LunarVApplicability::EarthShadowIntersection => "进入地影",
        _ => "其他适用状态",
    }
}

fn calculate_body(
    observer: FixedObserverAt<'_, Utc>,
    atmosphere: AtmosphericConditions,
    options: ReceptionLightTimeOptions,
    local_apparent_sidereal_hours: f64,
    name: &'static str,
    target: CelestialBody,
) -> BodySnapshot {
    match observer
        .vacuum_observed_place(target, options)
        .and_then(|vacuum| {
            vacuum
                .apply_refraction(atmosphere)
                .map(|observed| sky_snapshot(vacuum, observed, local_apparent_sidereal_hours))
        }) {
        Ok(sky) => BodySnapshot {
            name,
            sky: Some(sky),
            error: None,
        },
        Err(error) => BodySnapshot {
            name,
            sky: None,
            error: Some(error.to_string()),
        },
    }
}

fn sky_snapshot(
    vacuum: VacuumObservedPlace<Utc>,
    observed: ObservedPlace<Utc>,
    local_apparent_sidereal_hours: f64,
) -> SkyObjectSnapshot {
    let cirs = vacuum.intermediate_equatorial().coordinates();
    let vacuum_horizontal = vacuum.horizontal();
    let observed_horizontal = observed.horizontal();
    SkyObjectSnapshot {
        cirs_right_ascension_hours: cirs.right_ascension().as_hours(),
        cirs_declination_degrees: cirs.declination().as_degrees(),
        hour_angle_hours: canonical_hour_angle(
            local_apparent_sidereal_hours - cirs.right_ascension().as_hours(),
        ),
        azimuth_degrees: observed_horizontal
            .azimuth()
            .map(|value| value.as_degrees()),
        vacuum_altitude_degrees: vacuum_horizontal.altitude().as_degrees(),
        observed_altitude_degrees: observed_horizontal.altitude().as_degrees(),
        zenith_distance_degrees: 90.0 - observed_horizontal.altitude().as_degrees(),
        distance_astronomical_units: vacuum.distance().as_astronomical_units(),
        light_time_seconds: vacuum.light_time().as_seconds_f64(),
        light_time_iterations: vacuum.iterations(),
        light_time_residual_nanoseconds: vacuum.light_time_residual().as_nanoseconds(),
        refraction_arcseconds: observed.refraction().amount().as_degrees() * 3_600.0,
        refraction_accuracy: refraction_accuracy_label(observed.refraction().accuracy()),
    }
}

fn apparent_coordinates<S: TimeScale>(
    apparent: GeocentricApparentPlace<S>,
    orientation: CelestialOrientationSolution<S>,
) -> Result<ApparentCoordinatesSnapshot, FrameError> {
    let gcrs = apparent.gcrs_direction().coordinates();
    let j2000 = EquatorialDirection::<MeanEquatorEquinoxJ2000>::from_gcrs(gcrs)?;
    let mean_equatorial = orientation.mean_equatorial(gcrs)?.coordinates();
    let mean_ecliptic = orientation.mean_ecliptic_from_gcrs(gcrs)?.coordinates();
    let true_equatorial = apparent.true_equatorial().coordinates();
    let true_ecliptic = apparent.true_ecliptic().coordinates();
    Ok(ApparentCoordinatesSnapshot {
        gcrs_right_ascension_hours: gcrs.right_ascension().as_hours(),
        gcrs_declination_degrees: gcrs.declination().as_degrees(),
        j2000_right_ascension_hours: j2000.right_ascension().as_hours(),
        j2000_declination_degrees: j2000.declination().as_degrees(),
        mean_right_ascension_hours: mean_equatorial.right_ascension().as_hours(),
        mean_declination_degrees: mean_equatorial.declination().as_degrees(),
        true_right_ascension_hours: true_equatorial.right_ascension().as_hours(),
        true_declination_degrees: true_equatorial.declination().as_degrees(),
        true_ecliptic_longitude_degrees: true_ecliptic.longitude().as_degrees(),
        true_ecliptic_latitude_degrees: true_ecliptic.latitude().as_degrees(),
        mean_ecliptic_longitude_degrees: mean_ecliptic.longitude().as_degrees(),
        mean_ecliptic_latitude_degrees: mean_ecliptic.latitude().as_degrees(),
    })
}

fn format_datetime<S: TimeScale>(value: DateTime<Gregorian, S>) -> String {
    let date = value.date();
    let time = value.time();
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:09}",
        date.year(),
        date.month(),
        date.day(),
        time.hour(),
        time.minute(),
        time.second(),
        time.nanosecond(),
    )
}

fn format_date<S: TimeScale>(value: DateTime<Gregorian, S>) -> String {
    let date = value.date();
    format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day())
}

fn format_time_of_day(value: TimeOfDay) -> String {
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        value.hour(),
        value.minute(),
        value.second(),
        value.nanosecond() / 1_000_000,
    )
}

fn format_hours(hours: f64) -> String {
    let total_milliseconds = (hours.rem_euclid(24.0) * 3_600_000.0).round() as u64;
    let hours = total_milliseconds / 3_600_000;
    let minutes = total_milliseconds / 60_000 % 60;
    let seconds = total_milliseconds / 1_000 % 60;
    let milliseconds = total_milliseconds % 1_000;
    format!("{hours:02}:{minutes:02}:{seconds:02}.{milliseconds:03}")
}

fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(BACKGROUND)),
        area,
    );
    if area.width < 68 || area.height < 20 {
        render_too_small(frame, area);
        return;
    }

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(area);
    render_header(frame, app, sections[0]);
    render_tabs(frame, app, sections[1]);
    let body = sections[2].inner(Margin {
        horizontal: 1,
        vertical: 0,
    });
    match app.page {
        Page::Overview => render_overview(frame, &app.snapshot, body),
        Page::Sun => render_sun_page(frame, &app.snapshot, body),
        Page::Moon => render_moon_page(frame, &app.snapshot, body),
        Page::Bodies => render_bodies(frame, &app.snapshot, body),
        Page::Earth => render_earth(frame, &app.snapshot, body),
        Page::Observer => render_observer(frame, &app.snapshot, body),
        Page::SolarTerms => render_solar_terms(frame, &app.snapshot, body),
        Page::Uncertainty => render_uncertainty(frame, &app.snapshot, body),
    }
    render_footer(frame, app, sections[3]);
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(32),
            Constraint::Percentage(48),
            Constraint::Percentage(20),
        ])
        .split(area);
    let style = Style::default().bg(SURFACE).fg(TEXT);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " HYASTRO",
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" / 实时天文钟", Style::default().fg(TEXT)),
        ]))
        .style(style)
        .alignment(Alignment::Left),
        columns[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{} UTC", app.snapshot.clocks.utc),
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        ))
        .style(style)
        .alignment(Alignment::Center),
        columns[1],
    );
    let state = if app.paused {
        "已暂停"
    } else {
        "实时 · 1 Hz"
    };
    let state_color = if app.paused { AMBER } else { GREEN };
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{state}  "),
            Style::default()
                .fg(state_color)
                .add_modifier(Modifier::BOLD),
        ))
        .style(style)
        .alignment(Alignment::Right),
        columns[2],
    );
}

fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let labels = if area.width >= 110 {
        [
            "1 总览",
            "2 太阳",
            "3 月球",
            "4 太阳系",
            "5 地球与时间",
            "6 观测站",
            "7 节气",
            "8 误差",
        ]
    } else {
        [
            "1 总览", "2 太阳", "3 月球", "4 行星", "5 地时", "6 站点", "7 节气", "8 误差",
        ]
    };
    let tabs = Tabs::new(labels)
        .select(app.page.index())
        .style(Style::default().bg(SURFACE).fg(MUTED))
        .highlight_style(
            Style::default()
                .fg(CYAN)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
        .divider(Span::styled("  ", Style::default().bg(SURFACE)));
    frame.render_widget(tabs, area);
}

fn render_overview(frame: &mut Frame, snapshot: &Snapshot, area: Rect) {
    if area.width >= 110 {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(34),
                Constraint::Percentage(33),
                Constraint::Percentage(33),
            ])
            .split(area);
        let compact = area.height < 22;
        render_clock_panel(frame, snapshot, columns[0], compact);
        render_sun_panel(frame, snapshot, columns[1], compact);
        render_moon_panel(frame, snapshot, columns[2], compact);
    } else {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(6), Constraint::Min(10)])
            .split(area);
        render_clock_panel(frame, snapshot, rows[0], true);
        let luminaries = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[1]);
        render_sun_panel(frame, snapshot, luminaries[0], true);
        render_moon_panel(frame, snapshot, luminaries[1], true);
    }
}

fn render_sun_page(frame: &mut Frame, snapshot: &Snapshot, area: Rect) {
    if area.width >= 140 {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(31),
                Constraint::Percentage(31),
                Constraint::Percentage(38),
            ])
            .split(area);
        render_sun_panel(frame, snapshot, columns[0], false);
        render_coordinate_panel(
            frame,
            &snapshot.sun.sky,
            snapshot.sun.coordinates,
            &snapshot.solar_term,
            columns[1],
            "太阳 · 多坐标系",
            AMBER,
        );
        render_horizon_contacts_panel(
            frame,
            &snapshot.daily_events.sun,
            &snapshot.daily_events.local_date,
            columns[2],
            "太阳 · 升落对照",
            AMBER,
            false,
        );
    } else {
        render_horizon_contacts_panel(
            frame,
            &snapshot.daily_events.sun,
            &snapshot.daily_events.local_date,
            area,
            "太阳 · 升落对照",
            AMBER,
            true,
        );
    }
}

fn render_moon_page(frame: &mut Frame, snapshot: &Snapshot, area: Rect) {
    if area.width >= 140 {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(31),
                Constraint::Percentage(31),
                Constraint::Percentage(38),
            ])
            .split(area);
        render_moon_panel(frame, snapshot, columns[0], false);
        render_coordinate_panel(
            frame,
            &snapshot.moon.sky,
            snapshot.moon.coordinates,
            &snapshot.solar_term,
            columns[1],
            "月球 · 多坐标系",
            MOONLIGHT,
        );
        render_horizon_contacts_panel(
            frame,
            &snapshot.daily_events.moon,
            &snapshot.daily_events.local_date,
            columns[2],
            "月球 · 升落对照",
            MOONLIGHT,
            false,
        );
    } else {
        render_horizon_contacts_panel(
            frame,
            &snapshot.daily_events.moon,
            &snapshot.daily_events.local_date,
            area,
            "月球 · 升落对照",
            MOONLIGHT,
            true,
        );
    }
}

fn render_coordinate_panel(
    frame: &mut Frame,
    sky: &SkyObjectSnapshot,
    coordinates: ApparentCoordinatesSnapshot,
    solar_term: &SolarTermSnapshot,
    area: Rect,
    title: &str,
    accent: Color,
) {
    let lines = vec![
        metric(
            "GCRS 视赤经",
            &format_hours(coordinates.gcrs_right_ascension_hours),
            CYAN,
        ),
        metric(
            "GCRS 视赤纬",
            &format_dms_signed(coordinates.gcrs_declination_degrees),
            TEXT,
        ),
        blank_line(),
        metric(
            "J2000 平赤经",
            &format_hours(coordinates.j2000_right_ascension_hours),
            CYAN,
        ),
        metric(
            "J2000 平赤纬",
            &format_dms_signed(coordinates.j2000_declination_degrees),
            TEXT,
        ),
        blank_line(),
        metric(
            "日期平赤经",
            &format_hours(coordinates.mean_right_ascension_hours),
            CYAN,
        ),
        metric(
            "日期平赤纬",
            &format_dms_signed(coordinates.mean_declination_degrees),
            TEXT,
        ),
        blank_line(),
        metric(
            "日期真赤经",
            &format_hours(coordinates.true_right_ascension_hours),
            CYAN,
        ),
        metric(
            "日期真赤纬",
            &format_dms_signed(coordinates.true_declination_degrees),
            TEXT,
        ),
        blank_line(),
        metric(
            "CIRS 站心赤经",
            &format_hours(sky.cirs_right_ascension_hours),
            CYAN,
        ),
        metric(
            "CIRS 站心赤纬",
            &format_dms_signed(sky.cirs_declination_degrees),
            TEXT,
        ),
        blank_line(),
        metric(
            "日期平黄经",
            &format_dms_unsigned(coordinates.mean_ecliptic_longitude_degrees),
            accent,
        ),
        metric(
            "日期平黄纬",
            &format_dms_signed(coordinates.mean_ecliptic_latitude_degrees),
            TEXT,
        ),
        blank_line(),
        metric(
            "日期真黄经",
            &format_dms_unsigned(coordinates.true_ecliptic_longitude_degrees),
            accent,
        ),
        metric(
            "日期真黄纬",
            &format_dms_signed(coordinates.true_ecliptic_latitude_degrees),
            TEXT,
        ),
        blank_line(),
        metric("站心时角", &format_hours(sky.hour_angle_hours), CYAN),
        metric("站心方位角", &format_azimuth(sky.azimuth_degrees), TEXT),
        metric(
            "站心观测高度",
            &format_dms_signed(sky.observed_altitude_degrees),
            altitude_color(sky.observed_altitude_degrees),
        ),
        metric(
            "定气 · 当前",
            &format!(
                "{} · {} · ±0°（定义）",
                solar_term.current_name,
                format_dms_unsigned(solar_term.current_longitude_degrees)
            ),
            AMBER,
        ),
        metric(
            "始于 UTC+8",
            &format!(
                "{} · ±{:.6}s · |Δλ|={:.3e}″",
                solar_term.current_local_time,
                solar_term.current_time_uncertainty_seconds,
                solar_term.current_residual_arcseconds
            ),
            TEXT,
        ),
        metric(
            "定气 · 下一",
            &format!(
                "{} · {} · ±0°（定义）",
                solar_term.next_name,
                format_dms_unsigned(solar_term.next_longitude_degrees)
            ),
            AMBER,
        ),
        metric(
            "交于 UTC+8",
            &format!(
                "{} · ±{:.6}s · |Δλ|={:.3e}″",
                solar_term.next_local_time,
                solar_term.next_time_uncertainty_seconds,
                solar_term.next_residual_arcseconds
            ),
            TEXT,
        ),
        metric(
            "距下次",
            &format!(
                "{:.6} d · ±{:.9} d（数值）",
                solar_term.until_next_days, solar_term.until_next_uncertainty_days
            ),
            CYAN,
        ),
    ];
    render_panel(frame, area, title, accent, lines);
}

fn render_solar_terms(frame: &mut Frame, snapshot: &Snapshot, area: Rect) {
    let compact = area.width < 110;
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(12)])
        .split(area);
    let definition = if compact {
        "UTC+8 · 每项 ± 为求根时间误差；不含星历/EOP 模型误差"
    } else {
        "地心视太阳日期真黄经每 15° · UTC+8 · ± 为时间误差，|Δλ| 为黄经残差；不含 DE440/EOP 模型误差"
    };
    render_panel(
        frame,
        rows[0],
        "定气二十四节气",
        AMBER,
        vec![metric("定义", definition, TEXT)],
    );

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);
    let entries = &snapshot.solar_term_year.entries;
    render_solar_term_half(
        frame,
        columns[0],
        &entries[..12],
        1,
        &format!("{} · 小寒—夏至", snapshot.solar_term_year.year),
        snapshot,
        compact,
    );
    render_solar_term_half(
        frame,
        columns[1],
        &entries[12..],
        13,
        &format!("{} · 小暑—冬至", snapshot.solar_term_year.year),
        snapshot,
        compact,
    );
}

fn render_solar_term_half(
    frame: &mut Frame,
    area: Rect,
    entries: &[SolarTermEntrySnapshot],
    first_number: usize,
    title: &str,
    snapshot: &Snapshot,
    compact: bool,
) {
    let lines = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let color = if entry.name == snapshot.solar_term.current_name {
                AMBER
            } else if entry.name == snapshot.solar_term.next_name {
                CYAN
            } else {
                TEXT
            };
            if compact {
                compact_metric(
                    &format!("{:02} {}", first_number + index, entry.name),
                    &format!(
                        "{} ±{:.6}s",
                        entry.compact_local_time, entry.uncertainty_seconds
                    ),
                    color,
                )
            } else {
                metric(
                    &format!(
                        "{:02} {} {:03.0}°",
                        first_number + index,
                        entry.name,
                        entry.longitude_degrees
                    ),
                    &format!(
                        "{} · ±{:.6} s · |Δλ|={:.3e}″",
                        entry.local_time, entry.uncertainty_seconds, entry.residual_arcseconds
                    ),
                    color,
                )
            }
        })
        .collect();
    render_panel(frame, area, title, AMBER, lines);
}

fn render_clock_panel(frame: &mut Frame, snapshot: &Snapshot, area: Rect, compact: bool) {
    let clock = &snapshot.clocks;
    let lines = if compact {
        vec![
            metric("UTC", &clock.utc, TEXT),
            metric("UT1", &clock.ut1, CYAN),
            metric("地方视恒星时", &format_hours(clock.last_hours), CYAN),
            metric("视太阳时", &clock.apparent_solar, AMBER),
        ]
    } else {
        vec![
            metric("UTC", &clock.utc, TEXT),
            metric("TAI", &clock.tai, TEXT),
            metric("TT", &clock.tt, TEXT),
            metric("UT1", &clock.ut1, CYAN),
            metric("TDB", &clock.tdb, TEXT),
            blank_line(),
            metric("地方平恒星时", &format_hours(clock.lmst_hours), CYAN),
            metric("地方视恒星时", &format_hours(clock.last_hours), CYAN),
            metric("平太阳时", &clock.mean_solar, AMBER),
            metric("视太阳时", &clock.apparent_solar, AMBER),
            metric(
                "时差",
                &format!("{:+.4} min", clock.equation_of_time_minutes),
                TEXT,
            ),
            metric(
                "平黄赤交角",
                &format_dms_unsigned(snapshot.earth.mean_obliquity_degrees),
                TEXT,
            ),
            metric(
                "真黄赤交角",
                &format_dms_unsigned(snapshot.earth.true_obliquity_degrees),
                CYAN,
            ),
        ]
    };
    render_panel(frame, area, "观测站时间", CYAN, lines);
}

fn render_sun_panel(frame: &mut Frame, snapshot: &Snapshot, area: Rect, compact: bool) {
    let sun = &snapshot.sun;
    let sky = &sun.sky;
    let light_time = if compact {
        format!(
            "{:.3} s / {} 次",
            sky.light_time_seconds, sky.light_time_iterations
        )
    } else {
        format!(
            "{:.6} s / {} 次 / {} ns",
            sky.light_time_seconds, sky.light_time_iterations, sky.light_time_residual_nanoseconds
        )
    };
    let mut lines = vec![
        metric(
            "高度",
            &format!(
                "{}（观测）",
                format_dms_signed(sky.observed_altitude_degrees)
            ),
            altitude_color(sky.observed_altitude_degrees),
        ),
        metric(
            "真空高度",
            &format_dms_signed(sky.vacuum_altitude_degrees),
            TEXT,
        ),
        metric("方位角", &format_azimuth(sky.azimuth_degrees), TEXT),
        metric(
            "CIRS 站心赤经",
            &format_hours(sky.cirs_right_ascension_hours),
            TEXT,
        ),
        metric(
            "CIRS 站心赤纬",
            &format_dms_signed(sky.cirs_declination_degrees),
            TEXT,
        ),
        metric(
            "距离",
            &format!("{:.9} au", sky.distance_astronomical_units),
            TEXT,
        ),
        metric(
            "视直径",
            &format_dms_unsigned(sun.angular_diameter_arcminutes / 60.0),
            AMBER,
        ),
        metric("光行时", &light_time, TEXT),
        metric(
            "大气折射",
            &format!("{:+.3} arcsec", sky.refraction_arcseconds),
            TEXT,
        ),
        metric(
            "定气",
            &format!(
                "{} → {}",
                snapshot.solar_term.current_name, snapshot.solar_term.next_name
            ),
            AMBER,
        ),
    ];
    if !compact {
        lines.extend([
            metric("时角", &format_hours(sky.hour_angle_hours), CYAN),
            metric(
                "天顶距",
                &format_dms_unsigned(sky.zenith_distance_degrees),
                TEXT,
            ),
            metric(
                "GCRS 视赤经",
                &format_hours(sun.coordinates.gcrs_right_ascension_hours),
                CYAN,
            ),
            metric(
                "GCRS 视赤纬",
                &format_dms_signed(sun.coordinates.gcrs_declination_degrees),
                TEXT,
            ),
            metric(
                "日期真赤经",
                &format_hours(sun.coordinates.true_right_ascension_hours),
                CYAN,
            ),
            metric(
                "日期真赤纬",
                &format_dms_signed(sun.coordinates.true_declination_degrees),
                TEXT,
            ),
            metric(
                "日期真黄经",
                &format_dms_unsigned(sun.coordinates.true_ecliptic_longitude_degrees),
                TEXT,
            ),
            metric(
                "日期真黄纬",
                &format_dms_signed(sun.coordinates.true_ecliptic_latitude_degrees),
                TEXT,
            ),
            metric(
                "光线偏折",
                &format!(
                    "{:+.3}″ / {}",
                    sun.deflection_arcseconds, sun.deflection_disposition
                ),
                MUTED,
            ),
        ]);
    }
    render_panel(frame, area, "太阳", AMBER, lines);
}

fn render_moon_panel(frame: &mut Frame, snapshot: &Snapshot, area: Rect, compact: bool) {
    let moon = &snapshot.moon;
    let sky = &moon.sky;
    let mut lines = vec![
        metric(
            "高度",
            &format!(
                "{}（观测）",
                format_dms_signed(sky.observed_altitude_degrees)
            ),
            altitude_color(sky.observed_altitude_degrees),
        ),
        metric("方位角", &format_azimuth(sky.azimuth_degrees), TEXT),
        metric(
            "CIRS 站心赤经",
            &format_hours(sky.cirs_right_ascension_hours),
            TEXT,
        ),
        metric(
            "CIRS 站心赤纬",
            &format_dms_signed(sky.cirs_declination_degrees),
            TEXT,
        ),
        metric(
            "距离",
            &format!("{:.9} au", sky.distance_astronomical_units),
            TEXT,
        ),
        metric(
            "视直径",
            &format_dms_unsigned(moon.angular_diameter_arcminutes / 60.0),
            MOONLIGHT,
        ),
        metric(
            "照明比例",
            &format!("{:.4}% / {}", moon.illuminated_percent, moon.branch),
            MOONLIGHT,
        ),
        metric(
            "相位角",
            &format_dms_unsigned(moon.phase_angle_degrees),
            TEXT,
        ),
        metric(
            "视星等",
            &format!(
                "{:+.3} V / {}",
                moon.visual_magnitude, moon.magnitude_applicability
            ),
            MOONLIGHT,
        ),
    ];
    if !compact {
        lines.extend([
            metric("时角", &format_hours(sky.hour_angle_hours), CYAN),
            metric(
                "天顶距",
                &format_dms_unsigned(sky.zenith_distance_degrees),
                TEXT,
            ),
            metric(
                "GCRS 视赤经",
                &format_hours(moon.coordinates.gcrs_right_ascension_hours),
                CYAN,
            ),
            metric(
                "GCRS 视赤纬",
                &format_dms_signed(moon.coordinates.gcrs_declination_degrees),
                TEXT,
            ),
            metric(
                "日期真赤经",
                &format_hours(moon.coordinates.true_right_ascension_hours),
                CYAN,
            ),
            metric(
                "日期真赤纬",
                &format_dms_signed(moon.coordinates.true_declination_degrees),
                TEXT,
            ),
            metric(
                "星等修正",
                &format!(
                    "{:+.3} 距离 / {:+.3} 相位",
                    moon.magnitude_distance_correction, moon.magnitude_phase_correction
                ),
                TEXT,
            ),
            metric(
                "光通量比",
                &format!("{:.4e}", moon.flux_ratio_to_zero_magnitude),
                TEXT,
            ),
            metric(
                "距角",
                &format_dms_unsigned(moon.directed_elongation_degrees),
                TEXT,
            ),
            metric(
                "日月角距",
                &format_dms_unsigned(moon.apparent_separation_degrees),
                TEXT,
            ),
            metric(
                "日期真黄经",
                &format_dms_unsigned(moon.coordinates.true_ecliptic_longitude_degrees),
                TEXT,
            ),
            metric(
                "日期真黄纬",
                &format_dms_signed(moon.coordinates.true_ecliptic_latitude_degrees),
                TEXT,
            ),
            metric(
                "日光行时",
                &format!("{:.6} s", moon.sunlight_at_moon_seconds),
                TEXT,
            ),
        ]);
    }
    render_panel(frame, area, "月球", MOONLIGHT, lines);
}

fn render_horizon_contacts_panel(
    frame: &mut Frame,
    events: &BodyHorizonTimes,
    local_date: &str,
    area: Rect,
    title: &'static str,
    accent: Color,
    compact: bool,
) {
    let event_value = |value: &Option<String>| {
        if compact {
            event_time(value)
                .split(" · ")
                .next()
                .unwrap_or("-")
                .to_owned()
        } else {
            event_time(value).to_owned()
        }
    };
    let mut lines = vec![
        metric("当日事件", &format!("{local_date} UTC+8"), accent),
        metric("盘面模型", "天文地平线 · 动态球形视半径", MUTED),
        metric("折射前模型", "站心真空高度", CYAN),
        metric(
            "上缘接触升",
            &event_value(&events.vacuum.upper_limb_rise),
            TEXT,
        ),
        metric("中心升起", &event_value(&events.vacuum.rise), TEXT),
        metric(
            "下缘接触升",
            &event_value(&events.vacuum.lower_limb_rise),
            TEXT,
        ),
        metric(
            "下缘接触落",
            &event_value(&events.vacuum.lower_limb_set),
            TEXT,
        ),
        metric("中心落下", &event_value(&events.vacuum.set), TEXT),
        metric(
            "上缘接触落",
            &event_value(&events.vacuum.upper_limb_set),
            TEXT,
        ),
        blank_line(),
        metric("标准折射模型", "固定 34′ 地平折射 · 动态球形视半径", AMBER),
        metric(
            "上缘接触升",
            &event_value(&events.standard_refracted.upper_limb_rise),
            TEXT,
        ),
        metric(
            "中心升起",
            &event_value(&events.standard_refracted.rise),
            TEXT,
        ),
        metric(
            "下缘接触升",
            &event_value(&events.standard_refracted.lower_limb_rise),
            TEXT,
        ),
        metric(
            "下缘接触落",
            &event_value(&events.standard_refracted.lower_limb_set),
            TEXT,
        ),
        metric(
            "中心落下",
            &event_value(&events.standard_refracted.set),
            TEXT,
        ),
        metric(
            "上缘接触落",
            &event_value(&events.standard_refracted.upper_limb_set),
            TEXT,
        ),
    ];
    if !compact {
        lines.extend([
            blank_line(),
            metric("上中天", &event_value(&events.vacuum.upper_transit), TEXT),
        ]);
    }
    render_panel(frame, area, title, accent, lines);
}

fn render_bodies(frame: &mut Frame, snapshot: &Snapshot, area: Rect) {
    let wide = area.width >= 140;
    let header_cells = if wide {
        vec![
            "天体",
            "CIRS 赤经",
            "时角",
            "CIRS 赤纬",
            "方位角",
            "高度",
            "距离",
            "光行时",
            "状态",
            "各列误差",
        ]
    } else {
        vec!["天体", "CIRS 赤经", "CIRS 赤纬", "高度", "状态", "各列误差"]
    };
    let header = Row::new(header_cells)
        .style(Style::default().fg(MUTED).add_modifier(Modifier::BOLD))
        .bottom_margin(1);
    let rows = snapshot.bodies.iter().map(|body| body_row(body, wide));
    let widths = if wide {
        vec![
            Constraint::Length(15),
            Constraint::Length(11),
            Constraint::Length(13),
            Constraint::Length(15),
            Constraint::Length(14),
            Constraint::Length(15),
            Constraint::Length(13),
            Constraint::Length(11),
            Constraint::Length(8),
            Constraint::Min(9),
        ]
    } else {
        vec![
            Constraint::Length(14),
            Constraint::Length(11),
            Constraint::Length(15),
            Constraint::Length(15),
            Constraint::Length(8),
            Constraint::Min(9),
        ]
    };
    let table = Table::new(rows, widths)
        .header(header)
        .column_spacing(1)
        .block(panel("站心太阳系", CYAN));
    frame.render_widget(table, area);
}

fn body_row(body: &BodySnapshot, wide: bool) -> Row<'static> {
    let name_color = match body.name {
        "太阳" => AMBER,
        "月球" => MOONLIGHT,
        _ => TEXT,
    };
    let name = Cell::from(body.name).style(Style::default().fg(name_color));
    let Some(sky) = &body.sky else {
        let error = body.error.as_deref().unwrap_or("不可用").to_owned();
        let mut cells = vec![name, Cell::from(error).style(Style::default().fg(RED))];
        cells.resize_with(if wide { 10 } else { 6 }, || Cell::from("—"));
        return Row::new(cells);
    };
    let state = if sky.observed_altitude_degrees >= 0.0 {
        "地平线上"
    } else {
        "地平线下"
    };
    let state_color = altitude_color(sky.observed_altitude_degrees);
    let mut cells = vec![
        name,
        Cell::from(format_hours(sky.cirs_right_ascension_hours)),
    ];
    if wide {
        cells.push(Cell::from(format_hours(sky.hour_angle_hours)));
    }
    cells.push(Cell::from(format_dms_signed(sky.cirs_declination_degrees)));
    if wide {
        cells.push(Cell::from(format_azimuth(sky.azimuth_degrees)));
    }
    cells.push(
        Cell::from(format_dms_signed(sky.observed_altitude_degrees))
            .style(Style::default().fg(state_color)),
    );
    if wide {
        cells.push(Cell::from(format!(
            "{:.7} au",
            sky.distance_astronomical_units
        )));
        cells.push(Cell::from(format!("{:.2} s", sky.light_time_seconds)));
    }
    cells.push(
        Cell::from(state).style(
            Style::default()
                .fg(state_color)
                .add_modifier(Modifier::BOLD),
        ),
    );
    cells.push(Cell::from("σ—").style(Style::default().fg(MUTED)));
    Row::new(cells).style(Style::default().fg(TEXT))
}

fn render_earth(frame: &mut Frame, snapshot: &Snapshot, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    let clock = &snapshot.clocks;
    let time_lines = vec![
        metric("UTC", &clock.utc, TEXT),
        metric("TAI", &clock.tai, TEXT),
        metric("TT", &clock.tt, TEXT),
        metric("UT1", &clock.ut1, CYAN),
        metric("TDB", &clock.tdb, TEXT),
        blank_line(),
        metric("JD UTC", &format!("{:.9}", clock.jd_utc), TEXT),
        metric("MJD UTC", &format!("{:.9}", clock.mjd_utc), TEXT),
        metric("JD TT", &format!("{:.9}", clock.jd_tt), TEXT),
        metric("JD UT1", &format!("{:.9}", clock.jd_ut1), TEXT),
        metric("JD TDB", &format!("{:.9}", clock.jd_tdb), TEXT),
        metric(
            "ΔT",
            &format!("{:+.6} s", snapshot.earth.delta_t_seconds),
            TEXT,
        ),
    ];
    render_panel(frame, columns[0], "时间尺度", CYAN, time_lines);

    let earth = &snapshot.earth;
    let orientation_lines = vec![
        metric(
            "UT1−UTC",
            &format_with_standard_uncertainty(
                format!("{:+.7} s", earth.ut1_minus_utc_seconds),
                earth.ut1_minus_utc_standard_uncertainty_seconds,
                7,
                "s",
            ),
            CYAN,
        ),
        metric(
            "极移 xp",
            &format_with_standard_uncertainty(
                format!("{:+.6} arcsec", earth.polar_motion_x_arcseconds),
                earth.polar_motion_x_standard_uncertainty_arcseconds,
                6,
                "arcsec",
            ),
            TEXT,
        ),
        metric(
            "极移 yp",
            &format_with_standard_uncertainty(
                format!("{:+.6} arcsec", earth.polar_motion_y_arcseconds),
                earth.polar_motion_y_standard_uncertainty_arcseconds,
                6,
                "arcsec",
            ),
            TEXT,
        ),
        metric(
            "CIP dX",
            &format_with_standard_uncertainty(
                format!("{:+.3} mas", earth.pole_offset_x_milliarcseconds),
                earth.pole_offset_x_standard_uncertainty_milliarcseconds,
                3,
                "mas",
            ),
            TEXT,
        ),
        metric(
            "CIP dY",
            &format_with_standard_uncertainty(
                format!("{:+.3} mas", earth.pole_offset_y_milliarcseconds),
                earth.pole_offset_y_standard_uncertainty_milliarcseconds,
                3,
                "mas",
            ),
            TEXT,
        ),
        metric(
            "EOP σ 来源",
            uncertainty_origin_label(earth.eop_standard_uncertainty_origin),
            MUTED,
        ),
        metric(
            "CIP X / Y",
            &format!(
                "{:+.4} / {:+.4} arcsec",
                earth.cip_x_arcseconds, earth.cip_y_arcseconds
            ),
            TEXT,
        ),
        metric(
            "TIO s′",
            &format!("{:+.4} mas", earth.tio_locator_milliarcseconds),
            TEXT,
        ),
        metric("ERA", &format_hours(clock.era_hours), CYAN),
        metric("GMST", &format_hours(clock.gmst_hours), CYAN),
        metric("GAST", &format_hours(clock.gast_hours), CYAN),
        metric(
            "起源 / 分点方程",
            &format!(
                "{:+.4}″ / {:+.4}″",
                earth.equation_of_origins_arcseconds, earth.equation_of_equinoxes_arcseconds
            ),
            TEXT,
        ),
        metric("地方平恒星时", &format_hours(clock.lmst_hours), CYAN),
        metric("地方视恒星时", &format_hours(clock.last_hours), CYAN),
        metric(
            "黄赤交角",
            &format!(
                "{} 平 / {} 真",
                format_dms_unsigned(earth.mean_obliquity_degrees),
                format_dms_unsigned(earth.true_obliquity_degrees)
            ),
            TEXT,
        ),
        metric(
            "章动",
            &format!(
                "{:+.4}″ 黄经 / {:+.4}″ 交角",
                earth.nutation_longitude_arcseconds, earth.nutation_obliquity_arcseconds
            ),
            TEXT,
        ),
    ];
    render_panel(
        frame,
        columns[1],
        "地球定向 · IAU 2006/2000A",
        CYAN,
        orientation_lines,
    );
}

fn render_uncertainty(frame: &mut Frame, snapshot: &Snapshot, area: Rect) {
    let earth = &snapshot.earth;
    let data_lines = vec![
        metric(
            "UT1−UTC",
            &format_with_standard_uncertainty(
                format!("{:+.7} s", earth.ut1_minus_utc_seconds),
                earth.ut1_minus_utc_standard_uncertainty_seconds,
                7,
                "s",
            ),
            CYAN,
        ),
        metric(
            "极移 xp",
            &format_with_standard_uncertainty(
                format!("{:+.6} arcsec", earth.polar_motion_x_arcseconds),
                earth.polar_motion_x_standard_uncertainty_arcseconds,
                6,
                "arcsec",
            ),
            TEXT,
        ),
        metric(
            "极移 yp",
            &format_with_standard_uncertainty(
                format!("{:+.6} arcsec", earth.polar_motion_y_arcseconds),
                earth.polar_motion_y_standard_uncertainty_arcseconds,
                6,
                "arcsec",
            ),
            TEXT,
        ),
        metric(
            "CIP dX",
            &format_with_standard_uncertainty(
                format!("{:+.3} mas", earth.pole_offset_x_milliarcseconds),
                earth.pole_offset_x_standard_uncertainty_milliarcseconds,
                3,
                "mas",
            ),
            TEXT,
        ),
        metric(
            "CIP dY",
            &format_with_standard_uncertainty(
                format!("{:+.3} mas", earth.pole_offset_y_milliarcseconds),
                earth.pole_offset_y_standard_uncertainty_milliarcseconds,
                3,
                "mas",
            ),
            TEXT,
        ),
        metric(
            "σ 来源",
            uncertainty_origin_label(earth.eop_standard_uncertainty_origin),
            MUTED,
        ),
        metric("EOP 相关性", "源产品未提供 · 不构造完整协方差", AMBER),
    ];

    let solar_term = &snapshot.solar_term;
    let numerical_lines = vec![
        metric(
            "当日事件",
            &format!(
                "{} UTC+8 · 标准 34′ 折射天体中心（盘面接触见日月页）",
                snapshot.daily_events.local_date
            ),
            MUTED,
        ),
        metric(
            "太阳中心升",
            event_time(&snapshot.daily_events.sun.standard_refracted.rise),
            TEXT,
        ),
        metric(
            "太阳上中天",
            event_time(&snapshot.daily_events.sun.standard_refracted.upper_transit),
            TEXT,
        ),
        metric(
            "太阳中心落",
            event_time(&snapshot.daily_events.sun.standard_refracted.set),
            TEXT,
        ),
        metric(
            "月球中心升",
            event_time(&snapshot.daily_events.moon.standard_refracted.rise),
            TEXT,
        ),
        metric(
            "月球上中天",
            event_time(&snapshot.daily_events.moon.standard_refracted.upper_transit),
            TEXT,
        ),
        metric(
            "月球中心落",
            event_time(&snapshot.daily_events.moon.standard_refracted.set),
            TEXT,
        ),
        blank_line(),
        metric(
            "当前节气",
            &format!(
                "{} · ±{:.6}s · |Δλ|={:.3e}″",
                solar_term.current_name,
                solar_term.current_time_uncertainty_seconds,
                solar_term.current_residual_arcseconds,
            ),
            AMBER,
        ),
        metric(
            "下一节气",
            &format!(
                "{} · ±{:.6}s · |Δλ|={:.3e}″",
                solar_term.next_name,
                solar_term.next_time_uncertainty_seconds,
                solar_term.next_residual_arcseconds,
            ),
            AMBER,
        ),
    ];

    let coverage_lines = vec![
        metric("标准不确定度", "σ = 1σ · 与结果使用同一物理量", GREEN),
        metric("求根时间", "± = 最终括区间半宽", GREEN),
        metric("判据残差", "|Δf| / |Δλ| = 收敛后的函数残差", GREEN),
        blank_line(),
        metric(
            "星表协方差模型",
            "6 参数 J C Jᵀ 可用 · 当前快照无星表输入",
            GREEN,
        ),
        metric("即时天体方向", "σ— · EOP / 星历 / 大气尚未联合传播", AMBER),
        metric("DE440 模型误差", "σ— · BSP 未提供", AMBER),
        metric("大气输入误差", "σ— · 调用者未提供", AMBER),
    ];

    if area.width >= 130 {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ])
            .split(area);
        render_panel(frame, columns[0], "数据标准不确定度", CYAN, data_lines);
        render_panel(frame, columns[1], "数值求解证据", AMBER, numerical_lines);
        render_panel(frame, columns[2], "传播覆盖", GREEN, coverage_lines);
    } else if area.height >= 30 {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(9),
                Constraint::Length(13),
                Constraint::Min(8),
            ])
            .split(area);
        render_panel(frame, rows[0], "数据标准不确定度", CYAN, data_lines);
        render_panel(frame, rows[1], "数值求解证据", AMBER, numerical_lines);
        render_panel(frame, rows[2], "传播覆盖", GREEN, coverage_lines);
    } else {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        let compact_data = vec![
            data_lines[0].clone(),
            data_lines[1].clone(),
            data_lines[2].clone(),
            data_lines[3].clone(),
            data_lines[4].clone(),
            data_lines[5].clone(),
            blank_line(),
            coverage_lines[4].clone(),
            coverage_lines[5].clone(),
        ];
        let compact_numerical = vec![
            numerical_lines[0].clone(),
            numerical_lines[1].clone(),
            numerical_lines[2].clone(),
            numerical_lines[3].clone(),
            numerical_lines[4].clone(),
            numerical_lines[5].clone(),
            numerical_lines[6].clone(),
            numerical_lines[8].clone(),
            numerical_lines[9].clone(),
        ];
        render_panel(frame, columns[0], "标准不确定度与覆盖", CYAN, compact_data);
        render_panel(frame, columns[1], "数值求解证据", AMBER, compact_numerical);
    }
}

fn render_observer(frame: &mut Frame, snapshot: &Snapshot, area: Rect) {
    let observer = &snapshot.observer;
    let geometry_lines = vec![
        metric("纬度", &format_dms_signed(LATITUDE_DEGREES), CYAN),
        metric("经度", &format_dms_signed(LONGITUDE_DEGREES), CYAN),
        metric("椭球高", &format!("{HEIGHT_METRES:.3} m（椭球面）"), TEXT),
        metric("参考椭球", "WGS 84", TEXT),
        blank_line(),
        metric(
            "ITRS X",
            &format!("{:+.3} km", observer.itrs_kilometres[0]),
            TEXT,
        ),
        metric(
            "ITRS Y",
            &format!("{:+.3} km", observer.itrs_kilometres[1]),
            TEXT,
        ),
        metric(
            "ITRS Z",
            &format!("{:+.3} km", observer.itrs_kilometres[2]),
            TEXT,
        ),
    ];
    let kinematics_lines = vec![
        metric(
            "GCRS 位置 XY",
            &format!(
                "{:+.1} / {:+.1} km",
                observer.gcrs_kilometres[0], observer.gcrs_kilometres[1]
            ),
            TEXT,
        ),
        metric(
            "GCRS 位置 Z",
            &format!("{:+.1} km", observer.gcrs_kilometres[2]),
            TEXT,
        ),
        metric(
            "GCRS 速度 XY",
            &format!(
                "{:+.3} / {:+.3} m/s",
                observer.gcrs_velocity_metres_per_second[0],
                observer.gcrs_velocity_metres_per_second[1]
            ),
            TEXT,
        ),
        metric(
            "GCRS 速度 Z",
            &format!("{:+.3} m/s", observer.gcrs_velocity_metres_per_second[2]),
            TEXT,
        ),
        metric(
            "GCRS 速率",
            &format!("{:.6} m/s", observer.inertial_speed_metres_per_second),
            CYAN,
        ),
        blank_line(),
        metric(
            "BCRS 位置 XY",
            &format!(
                "{:+.6} / {:+.6} au",
                observer.bcrs_astronomical_units[0], observer.bcrs_astronomical_units[1]
            ),
            TEXT,
        ),
        metric(
            "BCRS 位置 Z",
            &format!("{:+.6} au", observer.bcrs_astronomical_units[2]),
            TEXT,
        ),
        metric(
            "BCRS 速度 XY",
            &format!(
                "{:+.6} / {:+.6} km/s",
                observer.bcrs_velocity_kilometres_per_second[0],
                observer.bcrs_velocity_kilometres_per_second[1]
            ),
            TEXT,
        ),
        metric(
            "BCRS 速度 Z",
            &format!(
                "{:+.6} km/s",
                observer.bcrs_velocity_kilometres_per_second[2]
            ),
            TEXT,
        ),
        metric(
            "BCRS 速率",
            &format!(
                "{:.6} km/s",
                observer.barycentric_speed_kilometres_per_second
            ),
            CYAN,
        ),
        metric("地球自转", observer.velocity_model, AMBER),
    ];
    let data = &snapshot.data;
    let status_lines = vec![
        metric("气压", &format!("{PRESSURE_HECTOPASCALS:.2} hPa"), TEXT),
        metric("温度", &format!("{TEMPERATURE_CELSIUS:.1} °C"), TEXT),
        metric(
            "相对湿度",
            &format!("{:.1}% RH", RELATIVE_HUMIDITY * 100.0),
            TEXT,
        ),
        metric("观测波长", &format!("{WAVELENGTH_MICROMETRES:.3} µm"), TEXT),
        metric("大气折射", "IAU SOFA refco / atioq", CYAN),
        metric("太阳折射精度", snapshot.sun.sky.refraction_accuracy, TEXT),
        metric("月球折射精度", snapshot.moon.sky.refraction_accuracy, TEXT),
        blank_line(),
        metric("EOP", "IERS finals2000A", TEXT),
        metric("覆盖范围", &data.eop_coverage, TEXT),
        metric(
            "星历",
            &format!("{} / {} 个内核", data.kernel_name, data.kernel_count),
            TEXT,
        ),
        metric("刷新周期", "1.000 s · 全量重算", GREEN),
        metric(
            "本轮计算",
            &format!("{:.3} ms", data.calculation_milliseconds),
            CYAN,
        ),
        metric("光行时", "延迟历元 · 1 ns 容差", TEXT),
        metric("未计算", "星表 · Shapiro 延迟", MUTED),
    ];

    if area.width >= 120 {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(29),
                Constraint::Percentage(36),
                Constraint::Percentage(35),
            ])
            .split(area);
        render_panel(frame, columns[0], "固定观测站", CYAN, geometry_lines);
        render_panel(frame, columns[1], "观测者运动学", CYAN, kinematics_lines);
        render_panel(frame, columns[2], "大气与数据", CYAN, status_lines);
    } else {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        let mut compact_motion = geometry_lines;
        compact_motion.extend([
            blank_line(),
            metric(
                "GCRS 速率",
                &format!("{:.6} m/s", observer.inertial_speed_metres_per_second),
                CYAN,
            ),
            metric(
                "BCRS 速率",
                &format!(
                    "{:.6} km/s",
                    observer.barycentric_speed_kilometres_per_second
                ),
                CYAN,
            ),
            metric("地球自转", observer.velocity_model, AMBER),
        ]);
        render_panel(frame, columns[0], "观测站与运动", CYAN, compact_motion);
        render_panel(frame, columns[1], "大气与数据", CYAN, status_lines);
    }
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let style = Style::default().bg(SURFACE).fg(MUTED);
    let line = if let Some(error) = &app.last_error {
        Line::from(vec![
            Span::styled(
                " 更新失败  ",
                Style::default().fg(RED).add_modifier(Modifier::BOLD),
            ),
            Span::styled(error.clone(), Style::default().fg(TEXT)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" Tab / ← → ", Style::default().fg(CYAN)),
            Span::raw("切页   "),
            Span::styled("空格", Style::default().fg(CYAN)),
            Span::raw(" 暂停   "),
            Span::styled("r", Style::default().fg(CYAN)),
            Span::raw(" 刷新   "),
            Span::styled("q", Style::default().fg(CYAN)),
            Span::raw(" 退出"),
            Span::raw("   "),
            Span::styled("σ—", Style::default().fg(AMBER)),
            Span::raw(" 物理/模型误差未传播"),
        ])
    };
    frame.render_widget(Paragraph::new(line).style(style), area);
}

fn render_too_small(frame: &mut Frame, area: Rect) {
    let block = panel("实时天文钟", RED);
    let message = Paragraph::new(vec![
        Line::from(Span::styled(
            "终端尺寸不足，无法完整显示仪表盘。",
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("当前：{}×{} · 至少需要：68×20", area.width, area.height),
            Style::default().fg(MUTED),
        )),
        Line::from(Span::styled(
            "请放大终端窗口；后台仍会每秒更新计算。",
            Style::default().fg(CYAN),
        )),
    ])
    .alignment(Alignment::Center)
    .block(block)
    .wrap(Wrap { trim: true });
    let centered = centered_rect(60, 7, area);
    frame.render_widget(message, centered);
}

fn render_panel(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    accent: Color,
    lines: Vec<Line<'static>>,
) {
    let paragraph = Paragraph::new(lines)
        .style(Style::default().bg(SURFACE).fg(TEXT))
        .block(panel(title, accent))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn panel(title: &str, accent: Color) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ))
        .padding(Padding::horizontal(1))
        .style(Style::default().bg(SURFACE))
}

fn metric(label: &str, value: &str, color: Color) -> Line<'static> {
    let padding = 15_usize.saturating_sub(Span::raw(label).width()).max(1);
    let value = annotate_uncertainty(label, value);
    Line::from(vec![
        Span::styled(
            format!("{label}{:padding$}", ""),
            Style::default().fg(MUTED),
        ),
        Span::styled(value, Style::default().fg(color)),
    ])
}

fn annotate_uncertainty(label: &str, value: &str) -> String {
    let metadata = label == "定义"
        || label == "EOP"
        || label == "星历"
        || label == "当日事件"
        || label == "可见性"
        || label == "定气"
        || label == "刷新周期"
        || label.contains("模型")
        || label.contains("版本")
        || label.contains("覆盖")
        || label.contains("参考");
    let contains_number = value.bytes().any(|byte| byte.is_ascii_digit());
    if !contains_number
        || metadata
        || value.contains('±')
        || value.contains("σ")
        || value.contains("容差")
    {
        value.to_owned()
    } else {
        format!("{value} σ—")
    }
}

fn compact_metric(label: &str, value: &str, color: Color) -> Line<'static> {
    let padding = 9_usize.saturating_sub(Span::raw(label).width()).max(1);
    Line::from(vec![
        Span::styled(
            format!("{label}{:padding$}", ""),
            Style::default().fg(MUTED),
        ),
        Span::styled(value.to_owned(), Style::default().fg(color)),
    ])
}

fn blank_line() -> Line<'static> {
    Line::from("")
}

fn altitude_color(altitude_degrees: f64) -> Color {
    if altitude_degrees >= 0.0 {
        GREEN
    } else {
        MUTED
    }
}

fn canonical_hour_angle(hours: f64) -> f64 {
    hours.rem_euclid(24.0)
}

fn format_dms_signed(degrees: f64) -> String {
    format_dms(degrees, true)
}

fn format_dms_unsigned(degrees: f64) -> String {
    format_dms(degrees, false)
}

fn format_with_standard_uncertainty(
    value: String,
    standard_uncertainty: Option<f64>,
    precision: usize,
    unit: &str,
) -> String {
    match standard_uncertainty {
        Some(uncertainty) => format!("{value} · σ={uncertainty:.precision$} {unit}"),
        None => value,
    }
}

fn uncertainty_origin_label(origin: Option<UncertaintyOrigin>) -> &'static str {
    match origin {
        Some(UncertaintyOrigin::SourceReported) => "IERS 源记录",
        Some(UncertaintyOrigin::CorrelationAgnosticLinearInterpolation) => {
            "相邻 IERS σ 的相关性无关线性上界"
        }
        Some(_) => "未知传播方法",
        None => "源数据未提供",
    }
}

fn format_dms(degrees: f64, signed: bool) -> String {
    let sign = if degrees.is_sign_negative() {
        "−"
    } else if signed {
        "+"
    } else {
        ""
    };
    let total_tenths = (degrees.abs() * 36_000.0).round() as u64;
    let whole_degrees = total_tenths / 36_000;
    let minutes = total_tenths / 600 % 60;
    let seconds = total_tenths / 10 % 60;
    let tenths = total_tenths % 10;
    format!("{sign}{whole_degrees:03}°{minutes:02}′{seconds:02}.{tenths}″")
}

fn event_time(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("—")
}

fn format_azimuth(azimuth_degrees: Option<f64>) -> String {
    azimuth_degrees.map_or_else(|| "未定义".to_owned(), format_dms_unsigned)
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

fn print_horizon_events(name: &str, local_date: &str, events: &BodyHorizonTimes) {
    for (reference, times) in [
        ("折射前（真空）", &events.vacuum),
        ("标准折射（34′）", &events.standard_refracted),
    ] {
        println!(
            "{name}{reference}升起 {local_date} UTC+8  上缘 {}  中心 {}  下缘 {}",
            event_time(&times.upper_limb_rise),
            event_time(&times.rise),
            event_time(&times.lower_limb_rise),
        );
        println!(
            "{name}{reference}落下 {local_date} UTC+8  下缘 {}  中心 {}  上缘 {}",
            event_time(&times.lower_limb_set),
            event_time(&times.set),
            event_time(&times.upper_limb_set),
        );
    }
    println!(
        "{name}上中天 {local_date} UTC+8  {}",
        event_time(&events.vacuum.upper_transit),
    );
}

fn print_plain(snapshot: &Snapshot) {
    println!("HYASTRO 实时天文钟");
    println!("误差说明     显式 ±/残差为数值证据；其余即时量 σ—，表示物理/模型误差尚未传播");
    println!(
        "观测站       {} N，{} E，{HEIGHT_METRES:.3} m，WGS 84",
        format_dms_unsigned(LATITUDE_DEGREES),
        format_dms_unsigned(LONGITUDE_DEGREES)
    );
    println!("UTC          {}", snapshot.clocks.utc);
    println!("TAI          {}", snapshot.clocks.tai);
    println!("TT           {}", snapshot.clocks.tt);
    println!("UT1          {}", snapshot.clocks.ut1);
    println!("TDB          {}", snapshot.clocks.tdb);
    println!("地方平恒星时 {}", format_hours(snapshot.clocks.lmst_hours));
    println!("地方视恒星时 {}", format_hours(snapshot.clocks.last_hours));
    println!("平太阳时     {}", snapshot.clocks.mean_solar);
    println!("视太阳时     {}", snapshot.clocks.apparent_solar);
    println!(
        "时差         {:+.6} min",
        snapshot.clocks.equation_of_time_minutes
    );
    println!(
        "黄赤交角     {} 平 / {} 真",
        format_dms_unsigned(snapshot.earth.mean_obliquity_degrees),
        format_dms_unsigned(snapshot.earth.true_obliquity_degrees)
    );
    println!(
        "UT1−UTC      {}",
        format_with_standard_uncertainty(
            format!("{:+.7} s", snapshot.earth.ut1_minus_utc_seconds),
            snapshot.earth.ut1_minus_utc_standard_uncertainty_seconds,
            7,
            "s",
        )
    );
    println!("ΔT           {:+.7} s", snapshot.earth.delta_t_seconds);
    println!(
        "极移 xp      {}",
        format_with_standard_uncertainty(
            format!("{:+.6} arcsec", snapshot.earth.polar_motion_x_arcseconds),
            snapshot
                .earth
                .polar_motion_x_standard_uncertainty_arcseconds,
            6,
            "arcsec",
        )
    );
    println!(
        "极移 yp      {}",
        format_with_standard_uncertainty(
            format!("{:+.6} arcsec", snapshot.earth.polar_motion_y_arcseconds),
            snapshot
                .earth
                .polar_motion_y_standard_uncertainty_arcseconds,
            6,
            "arcsec",
        )
    );
    println!(
        "天极偏移 dX  {}",
        format_with_standard_uncertainty(
            format!("{:+.3} mas", snapshot.earth.pole_offset_x_milliarcseconds),
            snapshot
                .earth
                .pole_offset_x_standard_uncertainty_milliarcseconds,
            3,
            "mas",
        )
    );
    println!(
        "天极偏移 dY  {}",
        format_with_standard_uncertainty(
            format!("{:+.3} mas", snapshot.earth.pole_offset_y_milliarcseconds),
            snapshot
                .earth
                .pole_offset_y_standard_uncertainty_milliarcseconds,
            3,
            "mas",
        )
    );
    println!(
        "EOP σ 来源   {}",
        uncertainty_origin_label(snapshot.earth.eop_standard_uncertainty_origin)
    );
    println!(
        "起源/分点方程 {:+.4} / {:+.4} arcsec",
        snapshot.earth.equation_of_origins_arcseconds,
        snapshot.earth.equation_of_equinoxes_arcseconds
    );
    println!(
        "太阳  CIRS赤经 {}  时角 {}  赤纬 {}  方位角 {}  高度 {}  天顶距 {}  距离 {:.9} au",
        format_hours(snapshot.sun.sky.cirs_right_ascension_hours),
        format_hours(snapshot.sun.sky.hour_angle_hours),
        format_dms_signed(snapshot.sun.sky.cirs_declination_degrees),
        format_azimuth(snapshot.sun.sky.azimuth_degrees),
        format_dms_signed(snapshot.sun.sky.observed_altitude_degrees),
        format_dms_unsigned(snapshot.sun.sky.zenith_distance_degrees),
        snapshot.sun.sky.distance_astronomical_units
    );
    println!(
        "定气         当前 {} {}（{}）  下一 {} {}（{}）  剩余 {:.6} d",
        snapshot.solar_term.current_name,
        format_dms_unsigned(snapshot.solar_term.current_longitude_degrees),
        snapshot.solar_term.current_local_time,
        snapshot.solar_term.next_name,
        format_dms_unsigned(snapshot.solar_term.next_longitude_degrees),
        snapshot.solar_term.next_local_time,
        snapshot.solar_term.until_next_days,
    );
    println!(
        "{} 定气二十四节气（UTC+8；± 为求根时间误差，|Δλ| 为黄经残差；不含 DE440/EOP 模型误差）",
        snapshot.solar_term_year.year
    );
    for (index, entry) in snapshot.solar_term_year.entries.iter().enumerate() {
        println!(
            "{:02} {:<4} λ={:>5.1}°  {}  ±{:.6} s  |Δλ|={:.3e} arcsec",
            index + 1,
            entry.name,
            entry.longitude_degrees,
            entry.local_time,
            entry.uncertainty_seconds,
            entry.residual_arcseconds,
        );
    }
    print_horizon_events(
        "太阳",
        &snapshot.daily_events.local_date,
        &snapshot.daily_events.sun,
    );
    println!(
        "月球  CIRS赤经 {}  时角 {}  赤纬 {}  方位角 {}  高度 {}  照明 {:.4}%  视星等 {:+.3} V",
        format_hours(snapshot.moon.sky.cirs_right_ascension_hours),
        format_hours(snapshot.moon.sky.hour_angle_hours),
        format_dms_signed(snapshot.moon.sky.cirs_declination_degrees),
        format_azimuth(snapshot.moon.sky.azimuth_degrees),
        format_dms_signed(snapshot.moon.sky.observed_altitude_degrees),
        snapshot.moon.illuminated_percent,
        snapshot.moon.visual_magnitude
    );
    print_horizon_events(
        "月球",
        &snapshot.daily_events.local_date,
        &snapshot.daily_events.moon,
    );
    println!(
        "月球测光 距离修正 {:+.4} mag  相位修正 {:+.4} mag  光通量比 {:.6e}",
        snapshot.moon.magnitude_distance_correction,
        snapshot.moon.magnitude_phase_correction,
        snapshot.moon.flux_ratio_to_zero_magnitude
    );
    println!(
        "观测者速率 GCRS {:.6} m/s  BCRS {:.6} km/s",
        snapshot.observer.inertial_speed_metres_per_second,
        snapshot.observer.barycentric_speed_kilometres_per_second
    );
    println!();
    println!(
        "天体             CIRS赤经      时角             CIRS赤纬         方位角           高度             距离(AU)"
    );
    for body in &snapshot.bodies {
        if let Some(sky) = &body.sky {
            println!(
                "{:<16} {:<13} {:<16} {:<16} {:<16} {:<16} {:.9}",
                body.name,
                format_hours(sky.cirs_right_ascension_hours),
                format_hours(sky.hour_angle_hours),
                format_dms_signed(sky.cirs_declination_degrees),
                format_azimuth(sky.azimuth_degrees),
                format_dms_signed(sky.observed_altitude_degrees),
                sky.distance_astronomical_units,
            );
        } else {
            println!(
                "{:<16} 不可用：{}",
                body.name,
                body.error.as_deref().unwrap_or("未知错误")
            );
        }
    }
    println!();
    println!("EOP          {}", snapshot.data.eop_version);
    println!("覆盖范围     {}", snapshot.data.eop_coverage);
    println!("星历         {}", snapshot.data.kernel_name);
    println!("自转模型     {}", snapshot.observer.velocity_model);
    println!(
        "本轮计算     {:.3} ms",
        snapshot.data.calculation_milliseconds
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hour_angle_wraps_into_the_non_negative_sidereal_day() {
        assert_eq!(canonical_hour_angle(-1.0), 23.0);
        assert_eq!(canonical_hour_angle(25.0), 1.0);
    }
}
