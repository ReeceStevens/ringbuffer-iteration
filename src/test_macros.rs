#[macro_export]
macro_rules! assert_almost_eq {
    ($left:expr, $right:expr, $epsilon:expr) => ({
        match(&$left, &$right, &$epsilon) {
            (left_val, right_val, epsilon_val) => {
                let diff = *left_val - *right_val;
                if (diff < 0_f32) && (-diff > *epsilon_val) {
                    panic!("assertion failed: `(abs(left - right) < epsilon)\n`\
                            left: {:?},\n\
                            right: {:?},\n\
                            epsilon: {:?},\n\
                            actual diff: {:?}",
                            left_val, right_val, epsilon_val, diff)
                } else if (diff > *epsilon_val) {
                    panic!("assertion failed: `(abs(left - right) < epsilon)`\n\
                            left: {:?},\n\
                            right: {:?},\n\
                            epsilon: {:?},\n\
                            actual diff: {:?}",
                            left_val, right_val, epsilon_val, diff)
                }

            }
        }
    });
}

