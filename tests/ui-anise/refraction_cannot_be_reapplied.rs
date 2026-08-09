use hyastro::{
    astro::{AtmosphericConditions, ObservedPlace},
    time::TimeScale,
};

fn apply_refraction_twice<S: TimeScale>(
    observed: ObservedPlace<S>,
    conditions: AtmosphericConditions,
) {
    let _ = observed.apply_refraction(conditions);
}

fn main() {}
