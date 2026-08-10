use core::f64::consts::TAU;

use libm::{atan2, cos, sin, sqrt};
use std::vec::Vec;

use crate::{
    earth::{Earth, GeodeticPosition, ReferenceEllipsoid},
    ephem::{EphemerisProvenance, EphemerisProvider},
    frame::{HorizontalDirection, Itrs},
    math::{Length, Point3},
    time::{DeltaT, Duration, Instant, TimeInterval, TimeScale},
};

use super::{
    BesselianElements, BesselianElementsPolynomial, BesselianLimbModel,
    CentralSolarEclipseCharacter, Error, Events, GlobalSolarEclipse, search::BracketedRootSearch,
};

/// Sampling and contact-refinement controls for one geographic central-eclipse path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobalSolarEclipsePathOptions {
    sample_step: Duration,
    contact_time_tolerance: Duration,
    max_contact_iterations: u32,
}

impl GlobalSolarEclipsePathOptions {
    /// Smallest supported geographic-path sampling cadence.
    pub const MIN_SAMPLE_STEP: Duration =
        Duration::from_nanoseconds(Duration::NANOSECONDS_PER_SECOND);
    /// Largest supported geographic-path sampling cadence.
    pub const MAX_SAMPLE_STEP: Duration =
        Duration::from_nanoseconds(30 * 60 * Duration::NANOSECONDS_PER_SECOND);
    /// Largest supported central-contact time tolerance.
    pub const MAX_CONTACT_TIME_TOLERANCE: Duration =
        Duration::from_nanoseconds(Duration::NANOSECONDS_PER_SECOND);

    /// Constructs validated path sampling and central-contact controls.
    pub fn new(
        sample_step: Duration,
        contact_time_tolerance: Duration,
        max_contact_iterations: u32,
    ) -> Result<Self, Error> {
        if sample_step < Self::MIN_SAMPLE_STEP || sample_step > Self::MAX_SAMPLE_STEP {
            return Err(Error::InvalidBesselianPathSampleStep {
                nanoseconds: sample_step.as_nanoseconds(),
                minimum_nanoseconds: Self::MIN_SAMPLE_STEP.as_nanoseconds(),
                maximum_nanoseconds: Self::MAX_SAMPLE_STEP.as_nanoseconds(),
            });
        }
        if contact_time_tolerance <= Duration::ZERO
            || contact_time_tolerance > Self::MAX_CONTACT_TIME_TOLERANCE
        {
            return Err(Error::InvalidSearchDuration {
                field: "solar-eclipse central-contact time tolerance",
                nanoseconds: contact_time_tolerance.as_nanoseconds(),
                maximum_nanoseconds: Self::MAX_CONTACT_TIME_TOLERANCE.as_nanoseconds(),
            });
        }
        if max_contact_iterations == 0 {
            return Err(Error::InvalidSearchLimit {
                field: "solar-eclipse central-contact iterations",
                value: max_contact_iterations,
            });
        }
        Ok(Self {
            sample_step,
            contact_time_tolerance,
            max_contact_iterations,
        })
    }

    /// Returns the interval between retained geographic path points.
    pub const fn sample_step(self) -> Duration {
        self.sample_step
    }

    /// Returns the requested C2/C3 time tolerance.
    pub const fn contact_time_tolerance(self) -> Duration {
        self.contact_time_tolerance
    }

    /// Returns the maximum Brent iterations for each central contact.
    pub const fn max_contact_iterations(self) -> u32 {
        self.max_contact_iterations
    }

    /// Returns two-minute path samples and millisecond central-contact refinement.
    pub const fn standard() -> Self {
        Self {
            sample_step: Duration::from_nanoseconds(120 * Duration::NANOSECONDS_PER_SECOND),
            contact_time_tolerance: Duration::from_nanoseconds(1_000_000),
            max_contact_iterations: 64,
        }
    }
}

impl Default for GlobalSolarEclipsePathOptions {
    fn default() -> Self {
        Self::standard()
    }
}

/// Second and third contact for a fixed point on a central eclipse line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolarEclipseCentralPhase<S: TimeScale> {
    second_contact: Instant<S>,
    third_contact: Instant<S>,
    duration: Duration,
}

impl<S: TimeScale> SolarEclipseCentralPhase<S> {
    /// Returns second contact, when annularity or totality begins.
    pub const fn second_contact(self) -> Instant<S> {
        self.second_contact
    }

    /// Returns third contact, when annularity or totality ends.
    pub const fn third_contact(self) -> Instant<S> {
        self.third_contact
    }

    /// Returns the exact nanosecond-rounded central-phase duration `C3-C2`.
    pub const fn duration(self) -> Duration {
        self.duration
    }
}

/// Geographic circumstances at one sampled instant on a central eclipse path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlobalSolarEclipsePathPoint<S: TimeScale> {
    instant: Instant<S>,
    centre_line: GeodeticPosition,
    northern_limit: GeodeticPosition,
    southern_limit: GeodeticPosition,
    boundary_geodesic_span: Length,
    path_width: Length,
    central_phase: SolarEclipseCentralPhase<S>,
    character: CentralSolarEclipseCharacter,
    sun_direction: HorizontalDirection,
}

impl<S: TimeScale> GlobalSolarEclipsePathPoint<S> {
    /// Returns the physical sampling instant.
    pub const fn instant(self) -> Instant<S> {
        self.instant
    }

    /// Returns the centre-line position on the selected reference ellipsoid.
    pub const fn centre_line(self) -> GeodeticPosition {
        self.centre_line
    }

    /// Returns the northern envelope of the moving central shadow.
    pub const fn northern_limit(self) -> GeodeticPosition {
        self.northern_limit
    }

    /// Returns the southern envelope of the moving central shadow.
    pub const fn southern_limit(self) -> GeodeticPosition {
        self.southern_limit
    }

    /// Returns the ellipsoidal geodesic distance between the two moving-shadow path envelopes at
    /// this sampling instant.
    pub const fn boundary_geodesic_span(self) -> Length {
        self.boundary_geodesic_span
    }

    /// Returns the central path's cross-track width.
    ///
    /// Unlike [`Self::boundary_geodesic_span`], this is measured perpendicular to the centre-line
    /// motion using the conventional Besselian path-width projection.
    pub const fn path_width(self) -> Length {
        self.path_width
    }

    /// Returns C2, C3, and their central-phase duration at the centre-line site.
    pub const fn central_phase(self) -> SolarEclipseCentralPhase<S> {
        self.central_phase
    }

    /// Returns the central-phase duration at the centre-line site.
    pub const fn central_duration(self) -> Duration {
        self.central_phase.duration
    }

    /// Returns whether this path cross-section is annular or total.
    pub const fn character(self) -> CentralSolarEclipseCharacter {
        self.character
    }

    /// Returns the unrefracted Sun direction on local east-north-up axes.
    pub const fn sun_direction(self) -> HorizontalDirection {
        self.sun_direction
    }
}

/// Sampled centre line, moving-shadow envelopes, widths, and central durations.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalSolarEclipsePath<S: TimeScale> {
    interval: TimeInterval<S>,
    points: Vec<GlobalSolarEclipsePathPoint<S>>,
    earth: Earth,
    limb_model: BesselianLimbModel,
    delta_t: DeltaT<S>,
    ephemeris: EphemerisProvenance,
    options: GlobalSolarEclipsePathOptions,
}

impl<S: TimeScale> GlobalSolarEclipsePath<S> {
    /// Returns the complete interval in which the shadow axis intersects Earth.
    pub const fn interval(&self) -> TimeInterval<S> {
        self.interval
    }

    /// Returns complete two-sided path-envelope samples in chronological order.
    ///
    /// Near sunrise and sunset, one moving-shadow envelope branch can be absent even though the
    /// shadow axis already intersects Earth. Such one-sided edge instants remain represented by
    /// [`Self::interval`] but are not synthesized as complete cross-sections here.
    pub fn points(&self) -> &[GlobalSolarEclipsePathPoint<S>] {
        &self.points
    }

    /// Returns the selected Earth reference ellipsoid.
    pub const fn earth(&self) -> Earth {
        self.earth
    }

    /// Returns the explicit Besselian solar and lunar radius convention.
    pub const fn limb_model(&self) -> BesselianLimbModel {
        self.limb_model
    }

    /// Returns the constant `TT-UT1` used over this short-lived path fit.
    pub const fn delta_t(&self) -> DeltaT<S> {
        self.delta_t
    }

    /// Returns the ephemeris model and data provenance inherited from the fit.
    pub const fn ephemeris(&self) -> &EphemerisProvenance {
        &self.ephemeris
    }

    /// Returns the sampling and contact-refinement controls.
    pub const fn options(&self) -> GlobalSolarEclipsePathOptions {
        self.options
    }
}

#[derive(Clone, Copy)]
struct PathVector([f64; 3]);

impl PathVector {
    const fn new(components: [f64; 3]) -> Self {
        Self(components)
    }

    fn plus(self, other: Self) -> Self {
        Self([
            self.0[0] + other.0[0],
            self.0[1] + other.0[1],
            self.0[2] + other.0[2],
        ])
    }

    fn scaled(self, factor: f64) -> Self {
        Self([self.0[0] * factor, self.0[1] * factor, self.0[2] * factor])
    }

    fn dot(self, other: Self) -> f64 {
        self.0[0] * other.0[0] + self.0[1] * other.0[1] + self.0[2] * other.0[2]
    }
}

#[derive(Clone, Copy)]
struct FundamentalBasis {
    x: PathVector,
    y: PathVector,
    z: PathVector,
}

impl FundamentalBasis {
    // IAU/SOFA's Earth-rotation rate relative to uniform ephemeris time.
    const EARTH_ROTATION_RADIANS_PER_SECOND: f64 = TAU * 1.002_737_811_911_354_6 / 86_400.0;

    fn from_elements<S: TimeScale>(elements: &BesselianElements<S>, delta_t_seconds: f64) -> Self {
        let declination = elements.d().as_radians();
        let hour_angle =
            elements.mu().as_radians() - Self::EARTH_ROTATION_RADIANS_PER_SECOND * delta_t_seconds;
        let sin_d = sin(declination);
        let cos_d = cos(declination);
        let sin_mu = sin(hour_angle);
        let cos_mu = cos(hour_angle);
        Self {
            x: PathVector::new([sin_mu, cos_mu, 0.0]),
            y: PathVector::new([-sin_d * cos_mu, sin_d * sin_mu, cos_d]),
            z: PathVector::new([cos_d * cos_mu, -cos_d * sin_mu, sin_d]),
        }
    }
}

struct GeographicPathSolver<'a, S: TimeScale> {
    polynomial: &'a BesselianElementsPolynomial<S>,
    earth: Earth,
    delta_t_seconds: f64,
    options: GlobalSolarEclipsePathOptions,
}

impl<'a, S: TimeScale> GeographicPathSolver<'a, S> {
    const LIMIT_ANGLE_SAMPLES: usize = 720;
    const LIMIT_DERIVATIVE_STEP: Duration = Duration::from_nanoseconds(500_000_000);
    const CONTACT_SCAN_STEP: Duration =
        Duration::from_nanoseconds(15 * Duration::NANOSECONDS_PER_SECOND);

    fn new(
        polynomial: &'a BesselianElementsPolynomial<S>,
        delta_t: DeltaT<S>,
        options: GlobalSolarEclipsePathOptions,
    ) -> Self {
        Self {
            polynomial,
            earth: polynomial.earth(),
            delta_t_seconds: delta_t.as_seconds(),
            options,
        }
    }

    fn point(&self, instant: Instant<S>) -> Result<GlobalSolarEclipsePathPoint<S>, Error> {
        let elements = self.polynomial.elements_at(instant)?;
        let basis = FundamentalBasis::from_elements(&elements, self.delta_t_seconds);
        let centre_vector = self.axis_surface_vector(&elements, basis).ok_or(
            Error::ShadowAxisDoesNotIntersectEarth {
                epoch_tai_nanoseconds: instant.tai_nanoseconds_since_1900(),
            },
        )?;
        let centre_line = self.geodetic_position(centre_vector)?;
        let (northern_limit, southern_limit) = self.path_limits(instant, &elements, basis)?;
        let boundary_geodesic_span =
            self.geodesic_distance(northern_limit, southern_limit, instant)?;
        let path_width = self.cross_track_path_width(&elements, centre_vector, basis)?;
        let central_phase = self.central_phase(instant, centre_vector)?;
        let zeta = centre_vector.dot(basis.z);
        let signed_core_radius =
            elements.l2().as_equatorial_radii() - zeta * elements.tan_f2().value();
        let character = if signed_core_radius <= 0.0 {
            CentralSolarEclipseCharacter::Total
        } else {
            CentralSolarEclipseCharacter::Annular
        };
        let sun_direction = Self::sun_direction(centre_line, basis.z)?;
        Ok(GlobalSolarEclipsePathPoint {
            instant,
            centre_line,
            northern_limit,
            southern_limit,
            boundary_geodesic_span,
            path_width,
            central_phase,
            character,
            sun_direction,
        })
    }

    fn axis_surface_vector(
        &self,
        elements: &BesselianElements<S>,
        basis: FundamentalBasis,
    ) -> Option<PathVector> {
        let origin = basis
            .x
            .scaled(elements.x().as_equatorial_radii())
            .plus(basis.y.scaled(elements.y().as_equatorial_radii()));
        self.near_surface_intersection(origin, basis.z)
    }

    fn core_surface_vector(
        &self,
        elements: &BesselianElements<S>,
        basis: FundamentalBasis,
        theta: f64,
    ) -> Option<PathVector> {
        let transverse = basis.x.scaled(cos(theta)).plus(basis.y.scaled(sin(theta)));
        let origin = basis
            .x
            .scaled(elements.x().as_equatorial_radii())
            .plus(basis.y.scaled(elements.y().as_equatorial_radii()))
            .plus(transverse.scaled(elements.l2().as_equatorial_radii()));
        let direction = basis.z.plus(transverse.scaled(-elements.tan_f2().value()));
        self.near_surface_intersection(origin, direction)
    }

    fn near_surface_intersection(
        &self,
        origin: PathVector,
        direction: PathVector,
    ) -> Option<PathVector> {
        let ellipsoid = self.earth.reference_ellipsoid();
        let polar_ratio =
            ellipsoid.semi_minor_axis().as_metres() / ellipsoid.semi_major_axis().as_metres();
        let inverse_polar_ratio_squared = 1.0 / (polar_ratio * polar_ratio);
        let quadratic = |left: PathVector, right: PathVector| {
            left.0[0] * right.0[0]
                + left.0[1] * right.0[1]
                + left.0[2] * right.0[2] * inverse_polar_ratio_squared
        };
        let a = quadratic(direction, direction);
        let b = 2.0 * quadratic(origin, direction);
        let c = quadratic(origin, origin) - 1.0;
        let discriminant = b * b - 4.0 * a * c;
        if discriminant < 0.0 {
            return None;
        }
        let root = (-b + sqrt(discriminant)) / (2.0 * a);
        Some(origin.plus(direction.scaled(root)))
    }

    fn cross_track_path_width(
        &self,
        elements: &BesselianElements<S>,
        centre_vector: PathVector,
        basis: FundamentalBasis,
    ) -> Result<Length, Error> {
        let ellipsoid = self.earth.reference_ellipsoid();
        let eccentricity_squared = ellipsoid.first_eccentricity_squared();
        let declination = elements.d().as_radians();
        let sin_declination = sin(declination);
        let cos_declination = cos(declination);
        let rho = sqrt(1.0 - eccentricity_squared * cos_declination * cos_declination);
        let eta = elements.y().as_equatorial_radii() / rho;
        let zeta = centre_vector.dot(basis.z);
        let derivatives = elements.derivatives();
        let x_rate = derivatives.x().as_per_second();
        let y_rate = derivatives.y().as_per_second();
        let declination_rate = derivatives.d().as_radians_per_second();
        let hour_angle_rate = derivatives.mu().as_radians_per_second();
        let xi_rate = hour_angle_rate
            * (-elements.y().as_equatorial_radii() * sin_declination + zeta * cos_declination);
        let eta_rate = hour_angle_rate * elements.x().as_equatorial_radii() * sin_declination
            - declination_rate * zeta;
        let relative_x_rate = x_rate - xi_rate;
        let relative_y_rate = y_rate - eta_rate;
        let relative_speed =
            sqrt(relative_x_rate * relative_x_rate + relative_y_rate * relative_y_rate);
        let along_track_projection = (elements.x().as_equatorial_radii() * relative_x_rate
            + eta * relative_y_rate)
            / relative_speed;
        let core_radius = elements.l2().as_equatorial_radii() - zeta * elements.tan_f2().value();
        let projection = sqrt(zeta * zeta + along_track_projection * along_track_projection);
        Length::from_metres(
            (2.0 * ellipsoid.semi_major_axis().as_metres() * core_radius / projection).abs(),
        )
        .map_err(Error::from)
    }

    fn geodetic_position(&self, vector: PathVector) -> Result<GeodeticPosition, Error> {
        let equatorial_radius = self
            .earth
            .reference_ellipsoid()
            .semi_major_axis()
            .as_metres();
        self.earth
            .geodetic_position(Point3::<Itrs>::new(
                Length::from_metres(vector.0[0] * equatorial_radius)?,
                Length::from_metres(vector.0[1] * equatorial_radius)?,
                Length::from_metres(vector.0[2] * equatorial_radius)?,
            ))
            .map_err(Error::from)
    }

    fn path_limits(
        &self,
        instant: Instant<S>,
        elements: &BesselianElements<S>,
        basis: FundamentalBasis,
    ) -> Result<(GeodeticPosition, GeodeticPosition), Error> {
        let step = TAU / Self::LIMIT_ANGLE_SAMPLES as f64;
        let mut roots = Vec::with_capacity(4);
        let mut previous_theta = 0.0;
        let mut previous_value = self.path_envelope_rate(instant, elements, basis, 0.0)?;
        for index in 1..=Self::LIMIT_ANGLE_SAMPLES {
            let theta = index as f64 * step;
            let wrapped_theta = if index == Self::LIMIT_ANGLE_SAMPLES {
                0.0
            } else {
                theta
            };
            let value = self.path_envelope_rate(instant, elements, basis, wrapped_theta)?;
            if previous_value == 0.0 || previous_value * value < 0.0 {
                let root = self.refine_limit_theta(
                    instant,
                    elements,
                    basis,
                    previous_theta,
                    theta,
                    previous_value,
                )?;
                let wrapped_root = root.rem_euclid(TAU);
                if roots.iter().all(|existing: &f64| {
                    let separation = (wrapped_root - *existing).abs();
                    separation.min(TAU - separation) > 1.0e-8
                }) {
                    roots.push(wrapped_root);
                }
            }
            previous_theta = theta;
            previous_value = value;
        }
        if roots.len() < 2 {
            return Err(Error::BesselianPathLimitsUnavailable {
                epoch_tai_nanoseconds: instant.tai_nanoseconds_since_1900(),
                found: roots.len(),
            });
        }
        let mut positions = roots
            .into_iter()
            .map(|theta| {
                self.core_surface_vector(elements, basis, theta)
                    .ok_or(Error::BesselianPathLimitsUnavailable {
                        epoch_tai_nanoseconds: instant.tai_nanoseconds_since_1900(),
                        found: 0,
                    })
                    .and_then(|vector| self.geodetic_position(vector))
            })
            .collect::<Result<Vec<_>, _>>()?;
        positions.sort_by(|left, right| {
            left.latitude()
                .as_radians()
                .total_cmp(&right.latitude().as_radians())
        });
        Ok((
            *positions.last().expect("at least two path-limit positions"),
            positions[0],
        ))
    }

    fn path_envelope_rate(
        &self,
        instant: Instant<S>,
        elements: &BesselianElements<S>,
        basis: FundamentalBasis,
        theta: f64,
    ) -> Result<f64, Error> {
        let site = self.core_surface_vector(elements, basis, theta).ok_or(
            Error::BesselianPathLimitsUnavailable {
                epoch_tai_nanoseconds: instant.tai_nanoseconds_since_1900(),
                found: 0,
            },
        )?;
        let before = instant.checked_sub(Self::LIMIT_DERIVATIVE_STEP)?;
        let after = instant.checked_add(Self::LIMIT_DERIVATIVE_STEP)?;
        Ok(self.fixed_site_core_residual(site, after)?
            - self.fixed_site_core_residual(site, before)?)
    }

    fn refine_limit_theta(
        &self,
        instant: Instant<S>,
        elements: &BesselianElements<S>,
        basis: FundamentalBasis,
        mut lower: f64,
        mut upper: f64,
        mut lower_value: f64,
    ) -> Result<f64, Error> {
        for _ in 0..56 {
            let midpoint = 0.5 * (lower + upper);
            let value =
                self.path_envelope_rate(instant, elements, basis, midpoint.rem_euclid(TAU))?;
            if lower_value * value <= 0.0 {
                upper = midpoint;
            } else {
                lower = midpoint;
                lower_value = value;
            }
        }
        Ok(0.5 * (lower + upper))
    }

    fn fixed_site_core_residual(
        &self,
        site: PathVector,
        instant: Instant<S>,
    ) -> Result<f64, Error> {
        let elements = self.polynomial.elements_at(instant)?;
        let basis = FundamentalBasis::from_elements(&elements, self.delta_t_seconds);
        let xi = site.dot(basis.x);
        let eta = site.dot(basis.y);
        let zeta = site.dot(basis.z);
        let u = elements.x().as_equatorial_radii() - xi;
        let v = elements.y().as_equatorial_radii() - eta;
        let radius = elements.l2().as_equatorial_radii() - zeta * elements.tan_f2().value();
        Ok(u * u + v * v - radius * radius)
    }

    fn central_phase(
        &self,
        instant: Instant<S>,
        site: PathVector,
    ) -> Result<SolarEclipseCentralPhase<S>, Error> {
        let second_bracket = self.contact_bracket(instant, site, false)?;
        let second = BracketedRootSearch::refine(
            second_bracket.0,
            second_bracket.1,
            self.options.contact_time_tolerance,
            self.options.max_contact_iterations,
            |epoch| self.fixed_site_core_residual(site, epoch),
        )?;
        let third_bracket = self.contact_bracket(instant, site, true)?;
        let third = BracketedRootSearch::refine(
            third_bracket.0,
            third_bracket.1,
            self.options.contact_time_tolerance,
            self.options.max_contact_iterations,
            |epoch| self.fixed_site_core_residual(site, epoch),
        )?;
        let second_contact = second.instant();
        let third_contact = third.instant();
        Ok(SolarEclipseCentralPhase {
            second_contact,
            third_contact,
            duration: third_contact.duration_since(second_contact)?,
        })
    }

    fn contact_bracket(
        &self,
        centre: Instant<S>,
        site: PathVector,
        forward: bool,
    ) -> Result<(Instant<S>, Instant<S>), Error> {
        let validity = self.polynomial.validity();
        let boundary = if forward {
            validity.end()
        } else {
            validity.start()
        };
        let mut inner = centre;
        let mut inner_value = self.fixed_site_core_residual(site, inner)?;
        while inner != boundary {
            let candidate = if forward {
                inner.checked_add(Self::CONTACT_SCAN_STEP)?.min(boundary)
            } else {
                inner.checked_sub(Self::CONTACT_SCAN_STEP)?.max(boundary)
            };
            let candidate_value = self.fixed_site_core_residual(site, candidate)?;
            if inner_value * candidate_value <= 0.0 {
                return if forward {
                    Ok((inner, candidate))
                } else {
                    Ok((candidate, inner))
                };
            }
            inner = candidate;
            inner_value = candidate_value;
        }
        Err(Error::BesselianCentralContactNotBracketed {
            contact: if forward { "C3" } else { "C2" },
            epoch_tai_nanoseconds: centre.tai_nanoseconds_since_1900(),
        })
    }

    fn sun_direction(
        position: GeodeticPosition,
        sun_axis: PathVector,
    ) -> Result<HorizontalDirection, Error> {
        let latitude = position.latitude().as_radians();
        let longitude = position.longitude().as_radians();
        let sin_latitude = sin(latitude);
        let cos_latitude = cos(latitude);
        let sin_longitude = sin(longitude);
        let cos_longitude = cos(longitude);
        let east = PathVector::new([-sin_longitude, cos_longitude, 0.0]);
        let north = PathVector::new([
            -sin_latitude * cos_longitude,
            -sin_latitude * sin_longitude,
            cos_latitude,
        ]);
        let up = PathVector::new([
            cos_latitude * cos_longitude,
            cos_latitude * sin_longitude,
            sin_latitude,
        ]);
        HorizontalDirection::from_enu_components([
            sun_axis.dot(east),
            sun_axis.dot(north),
            sun_axis.dot(up),
        ])
        .map_err(Error::from)
    }

    fn geodesic_distance(
        &self,
        first: GeodeticPosition,
        second: GeodeticPosition,
        instant: Instant<S>,
    ) -> Result<Length, Error> {
        let ellipsoid = self.earth.reference_ellipsoid();
        let distance = Self::vincenty_inverse(ellipsoid, first, second).ok_or(
            Error::BesselianPathWidthDidNotConverge {
                epoch_tai_nanoseconds: instant.tai_nanoseconds_since_1900(),
            },
        )?;
        Length::from_metres(distance).map_err(Error::from)
    }

    fn vincenty_inverse(
        ellipsoid: ReferenceEllipsoid,
        first: GeodeticPosition,
        second: GeodeticPosition,
    ) -> Option<f64> {
        let semi_major = ellipsoid.semi_major_axis().as_metres();
        let flattening = ellipsoid.flattening();
        let semi_minor = ellipsoid.semi_minor_axis().as_metres();
        let reduced_first = atan2(
            (1.0 - flattening) * sin(first.latitude().as_radians()),
            cos(first.latitude().as_radians()),
        );
        let reduced_second = atan2(
            (1.0 - flattening) * sin(second.latitude().as_radians()),
            cos(second.latitude().as_radians()),
        );
        let longitude_difference = second.longitude().as_radians() - first.longitude().as_radians();
        let (sin_u1, cos_u1) = (sin(reduced_first), cos(reduced_first));
        let (sin_u2, cos_u2) = (sin(reduced_second), cos(reduced_second));
        let mut lambda = longitude_difference;
        let mut converged = false;
        let mut sin_sigma = 0.0;
        let mut cos_sigma = 0.0;
        let mut sigma = 0.0;
        let mut cos_squared_alpha = 0.0;
        let mut cos_two_sigma_midpoint = 0.0;
        for _ in 0..64 {
            let sin_lambda = sin(lambda);
            let cos_lambda = cos(lambda);
            sin_sigma = sqrt(
                (cos_u2 * sin_lambda) * (cos_u2 * sin_lambda)
                    + (cos_u1 * sin_u2 - sin_u1 * cos_u2 * cos_lambda)
                        * (cos_u1 * sin_u2 - sin_u1 * cos_u2 * cos_lambda),
            );
            if sin_sigma == 0.0 {
                return Some(0.0);
            }
            cos_sigma = sin_u1 * sin_u2 + cos_u1 * cos_u2 * cos_lambda;
            sigma = atan2(sin_sigma, cos_sigma);
            let sin_alpha = cos_u1 * cos_u2 * sin_lambda / sin_sigma;
            cos_squared_alpha = 1.0 - sin_alpha * sin_alpha;
            cos_two_sigma_midpoint = if cos_squared_alpha <= f64::EPSILON {
                0.0
            } else {
                cos_sigma - 2.0 * sin_u1 * sin_u2 / cos_squared_alpha
            };
            let coefficient = flattening / 16.0
                * cos_squared_alpha
                * (4.0 + flattening * (4.0 - 3.0 * cos_squared_alpha));
            let next = longitude_difference
                + (1.0 - coefficient)
                    * flattening
                    * sin_alpha
                    * (sigma
                        + coefficient
                            * sin_sigma
                            * (cos_two_sigma_midpoint
                                + coefficient
                                    * cos_sigma
                                    * (-1.0
                                        + 2.0 * cos_two_sigma_midpoint * cos_two_sigma_midpoint)));
            if (next - lambda).abs() <= 1.0e-13 {
                lambda = next;
                converged = true;
                break;
            }
            lambda = next;
        }
        if !converged || !lambda.is_finite() {
            return None;
        }
        let squared_u = cos_squared_alpha * (semi_major * semi_major - semi_minor * semi_minor)
            / (semi_minor * semi_minor);
        let a = 1.0
            + squared_u / 16_384.0
                * (4_096.0 + squared_u * (-768.0 + squared_u * (320.0 - 175.0 * squared_u)));
        let b = squared_u / 1_024.0
            * (256.0 + squared_u * (-128.0 + squared_u * (74.0 - 47.0 * squared_u)));
        let delta_sigma = b
            * sin_sigma
            * (cos_two_sigma_midpoint
                + b / 4.0
                    * (cos_sigma * (-1.0 + 2.0 * cos_two_sigma_midpoint * cos_two_sigma_midpoint)
                        - b / 6.0
                            * cos_two_sigma_midpoint
                            * (-3.0 + 4.0 * sin_sigma * sin_sigma)
                            * (-3.0 + 4.0 * cos_two_sigma_midpoint * cos_two_sigma_midpoint)));
        Some(semi_minor * a * (sigma - delta_sigma))
    }

    fn sample_instants(
        &self,
        interval: TimeInterval<S>,
        greatest: Instant<S>,
    ) -> Result<Vec<Instant<S>>, Error> {
        let mut instants = Vec::new();
        let mut cursor = interval.start().checked_add(self.options.sample_step)?;
        while cursor < interval.end() {
            instants.push(cursor);
            cursor = cursor.checked_add(self.options.sample_step)?;
        }
        if interval.contains(greatest) {
            instants.push(greatest);
        }
        let reference_epoch = self.polynomial.reference_epoch();
        if interval.contains(reference_epoch) {
            instants.push(reference_epoch);
        }
        instants.sort_unstable();
        instants.dedup();
        Ok(instants)
    }
}

impl<'context, 'data, E, P: EphemerisProvider + ?Sized> Events<'context, 'data, E, P> {
    /// Computes a sampled geographic central path from one classified eclipse and polynomial.
    ///
    /// `delta_t` supplies one caller-resolved `TT-UT1` value at the polynomial reference epoch.
    /// It is held constant over the six-hour fit, while the polynomial's `mu` is converted from
    /// ephemeris time to the rotating Earth. Path limits are the envelope of the moving core
    /// shadow, not north/south extrema of one instantaneous shadow footprint. Width uses the
    /// inverse geodesic on the selected ellipsoid; duration refines C2 and C3 at each centre-line
    /// site. Solar altitude and azimuth are geometric and unrefracted. Sampling retains only
    /// complete cross-sections with both moving-shadow envelope branches; the returned interval
    /// still preserves the earlier/later shadow-axis tangencies.
    pub fn solar_eclipse_path<S: TimeScale>(
        &self,
        eclipse: &GlobalSolarEclipse<S>,
        polynomial: &BesselianElementsPolynomial<S>,
        delta_t: DeltaT<S>,
        options: GlobalSolarEclipsePathOptions,
    ) -> Result<GlobalSolarEclipsePath<S>, Error> {
        let central_path = eclipse
            .central_path()
            .ok_or(Error::SolarEclipseHasNoCentralPath)?;
        if eclipse.earth() != polynomial.earth() {
            return Err(Error::BesselianPathEarthMismatch);
        }
        let events_ephemeris = self.astrometry().ephemeris().provenance()?;
        if eclipse.ephemeris() != &events_ephemeris || polynomial.ephemeris() != &events_ephemeris {
            return Err(Error::BesselianPathEphemerisMismatch);
        }
        if delta_t.epoch() != polynomial.reference_epoch() {
            return Err(Error::BesselianPathDeltaTEpochMismatch {
                expected_tai_nanoseconds: polynomial.reference_epoch().tai_nanoseconds_since_1900(),
                actual_tai_nanoseconds: delta_t.epoch().tai_nanoseconds_since_1900(),
            });
        }
        let interval =
            TimeInterval::new(central_path.start().instant(), central_path.end().instant())?;
        let validity = polynomial.validity();
        if validity.start() > interval.start() || validity.end() < interval.end() {
            return Err(Error::BesselianPathValidityTooShort {
                validity_start_tai_nanoseconds: validity.start().tai_nanoseconds_since_1900(),
                validity_end_tai_nanoseconds: validity.end().tai_nanoseconds_since_1900(),
                path_start_tai_nanoseconds: interval.start().tai_nanoseconds_since_1900(),
                path_end_tai_nanoseconds: interval.end().tai_nanoseconds_since_1900(),
            });
        }
        let solver = GeographicPathSolver::new(polynomial, delta_t, options);
        let instants = solver.sample_instants(interval, eclipse.maximum().instant())?;
        let mut points = Vec::with_capacity(instants.len());
        for instant in instants {
            match solver.point(instant) {
                Ok(point) => points.push(point),
                Err(Error::BesselianPathLimitsUnavailable { .. }) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(GlobalSolarEclipsePath {
            interval,
            points,
            earth: polynomial.earth(),
            limb_model: polynomial.limb_model(),
            delta_t,
            ephemeris: polynomial.ephemeris().clone(),
            options,
        })
    }
}
