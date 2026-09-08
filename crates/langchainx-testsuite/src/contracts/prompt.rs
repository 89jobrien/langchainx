use langchainx_prompt::{PromptArgs, PromptFromatter};

/// Asserts that formatting valid named inputs yields exactly the expected text.
pub fn assert_prompt_format<F: PromptFromatter>(formatter: &F, inputs: PromptArgs, expected: &str) {
    let actual = formatter
        .format(inputs)
        .expect("PromptFromatter::format must succeed");
    assert_eq!(actual, expected);
}

/// Asserts that a formatter declares exactly the expected variables in order.
pub fn assert_prompt_variables<F: PromptFromatter>(formatter: &F, expected: &[&str]) {
    assert_eq!(formatter.variables(), expected);
}

/// Asserts that formatting fails when a declared input variable is absent.
pub fn assert_missing_prompt_input<F: PromptFromatter>(formatter: &F, inputs: PromptArgs) {
    assert!(
        formatter.format(inputs).is_err(),
        "PromptFromatter::format must reject missing variables"
    );
}
