//! Text prompt templates and construction macros.
use crate::schemas::{messages::Message, prompt::PromptValue};

use super::{FormatPrompter, PromptArgs, PromptError, PromptFromatter};

#[derive(Clone)]
/// Placeholder syntax used by a [`PromptTemplate`].
pub enum TemplateFormat {
    /// Single-brace placeholders such as `{name}`.
    FString,
    /// Double-brace placeholders such as `{{name}}`.
    Jinja2,
}

#[derive(Clone)]
/// A text template with an explicit set of required variables.
pub struct PromptTemplate {
    template: String,
    variables: Vec<String>,
    format: TemplateFormat,
}

impl PromptTemplate {
    /// Creates a template with its required variables and placeholder syntax.
    pub fn new(template: String, variables: Vec<String>, format: TemplateFormat) -> Self {
        Self {
            template,
            variables,
            format,
        }
    }
}

//PromptTemplate will be default transformed to an Human Input when used as FromatPrompter
impl FormatPrompter for PromptTemplate {
    fn format_prompt(&self, input_variables: PromptArgs) -> Result<PromptValue, PromptError> {
        let messages = vec![Message::new_human_message(self.format(input_variables)?)];
        Ok(PromptValue::from_messages(messages))
    }
    fn get_input_variables(&self) -> Vec<String> {
        self.variables.clone()
    }
}

impl PromptFromatter for PromptTemplate {
    fn template(&self) -> String {
        self.template.clone()
    }

    fn variables(&self) -> Vec<String> {
        self.variables.clone()
    }

    fn format(&self, input_variables: PromptArgs) -> Result<String, PromptError> {
        for key in self.variables() {
            if !input_variables.contains_key(key.as_str()) {
                return Err(PromptError::MissingVariable(key));
            }
        }

        let (opening, closing) = match self.format {
            TemplateFormat::FString => ("{", "}"),
            TemplateFormat::Jinja2 => ("{{", "}}"),
        };
        let prompt = render_template(
            &self.template,
            &self.variables,
            &input_variables,
            opening,
            closing,
        );

        log::debug!("Formatted prompt: {}", prompt);
        Ok(prompt)
    }
}

fn render_template(
    template: &str,
    variables: &[String],
    values: &PromptArgs,
    opening: &str,
    closing: &str,
) -> String {
    let mut rendered = String::with_capacity(template.len());
    let mut cursor = 0;

    while let Some(relative_start) = template[cursor..].find(opening) {
        let start = cursor + relative_start;
        rendered.push_str(&template[cursor..start]);
        let value_start = start + opening.len();
        let Some(relative_end) = template[value_start..].find(closing) else {
            rendered.push_str(&template[start..]);
            return rendered;
        };
        let end = value_start + relative_end;
        let key = &template[value_start..end];
        if variables.iter().any(|variable| variable == key) {
            if let Some(value) = values.get(key) {
                match value {
                    serde_json::Value::String(value) => rendered.push_str(value),
                    value => rendered.push_str(&value.to_string()),
                }
            }
        } else {
            rendered.push_str(&template[start..end + closing.len()]);
        }
        cursor = end + closing.len();
    }

    rendered.push_str(&template[cursor..]);
    rendered
}

/// Creates [`PromptArgs`](crate::prompt::PromptArgs) from serializable key-value pairs.
///
/// # Usage
/// In this macro, the keys are `&str` and values are arbitrary types that get serialized into `serde_json::Value`:
/// ```rust,ignore
/// prompt_args! {
///     "input" => "Who is the writer of 20,000 Leagues Under the Sea, and what is my name?",
///     "history" => vec![
///         Message::new_human_message("My name is: Luis"),
///         Message::new_ai_message("Hi Luis"),
///     ],
/// }
/// ```
///
/// # Arguments
/// * `key` - A `&str` that will be used as the key in the resulting HashMap.<br>
/// * `value` - An arbitrary type that will be serialized into `serde_json::Value` and associated with the corresponding key.
///
#[macro_export]
macro_rules! prompt_args {
    ( $($key:expr => $value:expr),* $(,)? ) => {
        {
            #[allow(unused_mut)]
            let mut args = std::collections::HashMap::<String, serde_json::Value>::new();
            $(
                // Convert the value to serde_json::Value before inserting
                args.insert($key.to_string(), serde_json::json!($value));
            )*
            args
        }
    };
}

/// Creates a [`PromptTemplate`](crate::prompt::PromptTemplate) using single-brace placeholders.
///
/// # Usage
/// The macro is called with a template string and a list of variables that exist in the template. For example:
/// ```rust,ignore
/// template_fstring!(
///     "Hello {name}",
///     "name"
/// )
/// ```
#[macro_export]
macro_rules! template_fstring {
    ($template:expr, $($var:expr),* $(,)?) => {
        $crate::prompt::PromptTemplate::new(
            $template.to_string(),
            vec![$($var.to_string()),*],
            $crate::prompt::TemplateFormat::FString,
        )
    };
}

/// Creates a [`PromptTemplate`](crate::prompt::PromptTemplate) using double-brace placeholders.
///
/// # Usage
/// The macro is called with a template string and a list of variables that exist in the template. For example:
/// ```rust,ignore
/// template_jinja2!(
///     "Hello {{name}}",
///     "name"
/// )
/// ```
#[macro_export]
macro_rules! template_jinja2 {
    ($template:expr, $($var:expr),* $(,)?) => {
        $crate::prompt::PromptTemplate::new(
            $template.to_string(),
            vec![$($var.to_string()),*],
            $crate::prompt::TemplateFormat::Jinja2,
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_format_jinja2_template() {
        let template = PromptTemplate::new(
            "Hello {{name}}!".to_string(),
            vec!["name".to_string()],
            TemplateFormat::Jinja2,
        );

        let input_variables = prompt_args! {};
        let result = template.format(input_variables);
        assert!(result.is_err());

        let input_variables = prompt_args! {
            "name" => "world",
        };
        let result = template.format(input_variables);
        println!("{:?}", result);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello world!");
    }

    #[test]
    fn should_format_fstring_template() {
        let template = PromptTemplate::new(
            "Hello {name}!".to_string(),
            vec!["name".to_string()],
            TemplateFormat::FString,
        );

        let input_variables = prompt_args! {};
        let result = template.format(input_variables);
        assert!(result.is_err());

        let input_variables = prompt_args! {
            "name" => "world",
        };
        let result = template.format(input_variables);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello world!");
    }

    #[test]
    fn prompt_args_macro_empty_and_single_entry() {
        let args = prompt_args! {};
        assert!(args.is_empty());

        let args = prompt_args! {
            "name" => "world",
        };
        assert_eq!(args.len(), 1);
        assert_eq!(args.get("name").unwrap(), &"world");

        let args = prompt_args! {
            "name" => "world",
            "age" => "18",
        };
        assert_eq!(args.len(), 2);
        assert_eq!(args.get("name").unwrap(), &"world");
        assert_eq!(args.get("age").unwrap(), &"18");
    }

    #[test]
    fn test_chat_template_macros() {
        // Creating an FString chat template
        let fstring_template = template_fstring!(
            "FString Chat: {user} says {message} {test}",
            "user",
            "message",
            "test"
        );

        // Creating a Jinja2 chat template
        let jinja2_template =
            template_jinja2!("Jinja2 Chat: {{user}} says {{message}}", "user", "message");

        // Define input variables for the templates
        let input_variables_fstring = prompt_args! {
            "user" => "Alice",
            "message" => "Hello, Bob!",
            "test"=>"test2"
        };

        let input_variables_jinja2 = prompt_args! {
            "user" => "Bob",
            "message" => "Hi, Alice!",
        };

        // Format the FString chat template
        let formatted_fstring = fstring_template.format(input_variables_fstring).unwrap();
        assert_eq!(
            formatted_fstring,
            "FString Chat: Alice says Hello, Bob! test2"
        );

        // Format the Jinja2 chat template
        let formatted_jinja2 = jinja2_template.format(input_variables_jinja2).unwrap();
        assert_eq!(formatted_jinja2, "Jinja2 Chat: Bob says Hi, Alice!");
    }

    #[test]
    fn format_missing_variable_returns_error() {
        let template = template_fstring!("Hello {name} you are {age}", "name", "age");
        let args = prompt_args! { "name" => "Alice" };
        let result = template.format(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, PromptError::MissingVariable(ref v) if v == "age"),
            "expected MissingVariable(age), got: {err:?}"
        );
    }

    #[test]
    fn format_empty_template_returns_empty_string() {
        let template = template_fstring!("",);
        let args = prompt_args! {};
        let result = template.format(args).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn format_with_unicode_and_special_chars() {
        let template = template_fstring!("Caf\u{00e9} {drink} \u{1F600}", "drink");
        let args = prompt_args! { "drink" => "latte" };
        let result = template.format(args).unwrap();
        assert_eq!(result, "Caf\u{00e9} latte \u{1F600}");
    }

    #[test]
    fn format_does_not_replace_undeclared_placeholders() {
        let template = template_fstring!("Hello {name}; keep {internal}", "name");
        let args = prompt_args! {
            "name" => "Alice",
            "internal" => "secret",
        };

        let result = template.format(args).unwrap();

        assert_eq!(result, "Hello Alice; keep {internal}");
    }

    #[test]
    fn format_does_not_reprocess_inserted_placeholders() {
        let template = PromptTemplate::new(
            "{first}".to_string(),
            vec!["first".to_string(), "second".to_string()],
            TemplateFormat::FString,
        );
        let args = prompt_args! {
            "first" => "{second}",
            "second" => "resolved",
        };

        let result = template.format(args).unwrap();

        assert_eq!(result, "{second}");
    }

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn fstring_roundtrip_preserves_literal(s in "[^{}]*") {
                // A template with no variables should format to itself.
                let template = template_fstring!(&s,);
                let args = prompt_args! {};
                let result = template.format(args).unwrap();
                prop_assert_eq!(result, s);
            }
        }
    }
}
