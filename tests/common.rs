pub fn approx_equal<T: Into<f64>>(a: T, b: T) -> bool {
    return (a.into() - b.into()).abs() < 0.00001f64;
}
