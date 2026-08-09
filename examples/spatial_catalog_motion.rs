use std::error::Error;

use hyastro::{
    catalog::{
        CatalogProperMotion, CatalogRadialVelocity, Parallax, SpatialCatalogCovariance,
        SpatialCatalogParameter, SpatialCatalogPlace, SpatialCatalogStandardUncertainties,
    },
    frame::{EquatorialDirection, Icrs},
    math::{Angle, Declination, RightAscension, Speed},
    time::{JulianDate, Tcb},
    uncertainty::{CorrelationMatrix, StandardUncertainty},
};

fn main() -> Result<(), Box<dyn Error>> {
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
    let radians_per_milliarcsecond = core::f64::consts::PI / 648_000_000.0;
    let proper_motion_uncertainties =
        CatalogProperMotion::from_milliarcseconds_per_julian_year(0.08, 0.06)?;
    let standard_uncertainties = SpatialCatalogStandardUncertainties::new(
        StandardUncertainty::new(Angle::from_radians(0.12 * radians_per_milliarcsecond)?)?,
        StandardUncertainty::new(Angle::from_radians(0.10 * radians_per_milliarcsecond)?)?,
        StandardUncertainty::new(Angle::from_radians(0.05 * radians_per_milliarcsecond)?)?,
        StandardUncertainty::new(proper_motion_uncertainties.right_ascension_cos_declination())?,
        StandardUncertainty::new(proper_motion_uncertainties.declination())?,
        StandardUncertainty::new(Speed::from_metres_per_second(300.0)?)?,
    );
    let mut correlation_coefficients = CorrelationMatrix::<6>::identity().coefficients();
    correlation_coefficients[0][1] = -0.20;
    correlation_coefficients[1][0] = -0.20;
    correlation_coefficients[2][3] = 0.30;
    correlation_coefficients[3][2] = 0.30;
    let covariance = SpatialCatalogCovariance::new(
        standard_uncertainties,
        CorrelationMatrix::try_from_coefficients(correlation_coefficients)?,
    )?;
    let propagation_epoch = JulianDate::<Tcb>::from_parts(2_400_000.5, 53_736.0)?;
    let covariance_propagation = catalog
        .with_covariance(covariance)?
        .propagate_to(propagation_epoch)?;
    let propagated_covariance = covariance_propagation.result().covariance();
    let propagated_uncertainties = propagated_covariance.standard_uncertainties();

    let state = catalog.barycentric_state()?;
    let round_trip = state.catalog_place()?;
    let propagated = catalog.propagate_to(propagation_epoch)?;
    let restored = propagated.propagate_to(catalog.reference_epoch())?;
    let reversal_error = catalog
        .direction()
        .separation_to(restored.direction())?
        .as_radians();

    println!(
        "state position = [{:.9}, {:.9}, {:.9}] au",
        state.position().x().as_astronomical_units(),
        state.position().y().as_astronomical_units(),
        state.position().z().as_astronomical_units(),
    );
    println!(
        "round-trip parallax = {:.9} arcsec, radial velocity = {:+.9} km/s",
        round_trip.parallax().as_arcseconds(),
        round_trip.radial_velocity().as_kilometres_per_second(),
    );
    println!(
        "propagated RA = {:.15} rad, Dec = {:.15} rad",
        propagated.direction().right_ascension().as_radians(),
        propagated.direction().declination().as_radians(),
    );
    println!("forward/reverse direction error = {reversal_error:.3e} rad");
    println!(
        "propagated sigma(alpha*, parallax) = {:.6}, {:.6} mas",
        propagated_uncertainties
            .right_ascension_tangent_plane()
            .value()
            .as_radians()
            / radians_per_milliarcsecond,
        propagated_uncertainties.parallax().value().as_radians() / radians_per_milliarcsecond,
    );
    println!(
        "propagated corr(parallax, mu_alpha*) = {:+.6}",
        propagated_covariance.correlation(
            SpatialCatalogParameter::Parallax,
            SpatialCatalogParameter::RightAscensionProperMotion,
        ),
    );
    println!(
        "d(alpha*_out)/d(mu_alpha*_in) = {:.6e} s",
        covariance_propagation.jacobian().canonical_derivative(
            SpatialCatalogParameter::RightAscensionTangentPlane,
            SpatialCatalogParameter::RightAscensionProperMotion,
        ),
    );

    Ok(())
}
