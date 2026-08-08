use hyastro::{
    frame::Gcrs,
    math::{Angle, Direction, Length, Matrix3, Quaternion, RootOptions, Rotation, Vector3},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vector = Vector3::<Gcrs, Length>::new(
        Length::from_metres(2.0)?,
        Length::from_metres(0.0)?,
        Length::from_metres(0.0)?,
    );
    let quarter_turn = Angle::from_degrees(90.0)?;
    let rotation = Rotation::<Gcrs, Gcrs>::around_z(quarter_turn)?;
    let rotated = rotation.apply_vector(vector)?;

    assert!(rotated.x().as_metres().abs() < 3.0e-16);
    assert!((rotated.y().as_metres() - 2.0).abs() < 3.0e-16);

    let axis = Direction::<Gcrs>::try_from_components([0.0, 0.0, 1.0])?;
    let quaternion = Quaternion::from_axis_angle(axis, quarter_turn)?;
    let halfway = Quaternion::identity().slerp(quaternion, 0.5)?;
    let halfway_vector = halfway.apply_vector(vector)?;
    let restored = quaternion
        .inverse()
        .apply_vector(quaternion.apply_vector(vector)?)?;

    assert!((halfway_vector.x().as_metres() - 2.0_f64.sqrt()).abs() < 1.0e-14);
    assert!((halfway_vector.y().as_metres() - 2.0_f64.sqrt()).abs() < 1.0e-14);
    assert!((restored.x().as_metres() - 2.0).abs() < 1.0e-14);

    let matrix = Matrix3::try_from_rows([[2.0, -1.0, 0.0], [1.0, 2.0, 1.0], [0.0, 3.0, 1.0]])?;
    let inverse = matrix.inverse()?;
    let identity = matrix.checked_mul(inverse)?;
    assert!(identity.orthogonality_residual()? < 1.0e-14);

    let root_options = RootOptions::new(1.0e-14, 1.0e-14, 100)?;
    let fixed_point = root_options.bisect(0.0, 1.0, |x| x.cos() - x)?;
    assert!(fixed_point.residual().abs() <= 1.0e-14);

    println!("rotation matrix = {:?}", rotation.matrix().rows());
    println!("quaternion      = {:?}", quaternion.components());
    println!(
        "45° vector      = [{:.9}, {:.9}, {:.9}] m",
        halfway_vector.x().as_metres(),
        halfway_vector.y().as_metres(),
        halfway_vector.z().as_metres()
    );
    println!(
        "cos(x)=x root   = {:.15}, residual={:.3e}, iterations={}",
        fixed_point.root(),
        fixed_point.residual(),
        fixed_point.iterations()
    );

    Ok(())
}
