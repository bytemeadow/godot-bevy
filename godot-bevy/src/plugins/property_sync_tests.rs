#[cfg(test)]
mod tests {
    use super::equal;

    #[test]
    fn floats_compare_representations() {
        assert!(equal(&f32::NAN, &f32::NAN));
        assert!(equal(&f64::NAN, &f64::NAN));
        assert!(!equal(&0.0f32, &-0.0f32));
        assert!(!equal(&0.0f64, &-0.0f64));
        assert!(!equal(&f32::from_bits(0x7fc00001), &f32::NAN));
        assert!(equal(&String::from("same"), &String::from("same")));
        assert!(!equal(&1, &2));
    }
}
