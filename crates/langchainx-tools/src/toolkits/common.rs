use std::collections::HashSet;

pub(crate) const MAX_TOOL_NAME_BYTES: usize = 64;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ToolNameError {
    Empty,
    NonAscii,
    TooLong { limit: usize },
    Duplicate(String),
}

pub(crate) fn normalize_tool_name(
    name: &str,
    prefix: Option<&str>,
) -> Result<String, ToolNameError> {
    let name = normalize_component(name)?;
    let Some(prefix) = prefix else {
        return Ok(name);
    };
    let prefix = normalize_component(prefix)?;
    let combined_len = prefix.len() + 1 + name.len();
    if combined_len > MAX_TOOL_NAME_BYTES {
        return Err(ToolNameError::TooLong {
            limit: MAX_TOOL_NAME_BYTES,
        });
    }

    Ok(format!("{prefix}_{name}"))
}

pub(crate) fn normalize_unique_tool_names<I, S>(
    names: I,
    prefix: Option<&str>,
) -> Result<Vec<String>, ToolNameError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut normalized_names = Vec::new();
    let mut seen = HashSet::new();

    for name in names {
        let normalized = normalize_tool_name(name.as_ref(), prefix)?;
        if !seen.insert(normalized.clone()) {
            return Err(ToolNameError::Duplicate(normalized));
        }
        normalized_names.push(normalized);
    }

    Ok(normalized_names)
}

fn normalize_component(component: &str) -> Result<String, ToolNameError> {
    if component.len() > MAX_TOOL_NAME_BYTES {
        return Err(ToolNameError::TooLong {
            limit: MAX_TOOL_NAME_BYTES,
        });
    }
    if !component.is_ascii() {
        return Err(ToolNameError::NonAscii);
    }

    let mut normalized = String::with_capacity(component.len());
    let mut separator_pending = false;
    let mut previous_alphanumeric = None;

    for (index, byte) in component.bytes().enumerate() {
        if byte.is_ascii_alphanumeric() {
            if separator_pending && !normalized.is_empty() {
                normalized.push('_');
            } else if byte.is_ascii_uppercase() && !normalized.is_empty() {
                let next_is_lowercase = component
                    .as_bytes()
                    .get(index + 1)
                    .is_some_and(u8::is_ascii_lowercase);
                let starts_word = previous_alphanumeric.is_some_and(|previous: u8| {
                    previous.is_ascii_lowercase()
                        || previous.is_ascii_digit()
                        || (previous.is_ascii_uppercase() && next_is_lowercase)
                });
                if starts_word {
                    normalized.push('_');
                }
            }
            normalized.push(char::from(byte.to_ascii_lowercase()));
            separator_pending = false;
            previous_alphanumeric = Some(byte);
        } else {
            separator_pending = !normalized.is_empty();
            previous_alphanumeric = None;
        }
    }

    if normalized.is_empty() {
        return Err(ToolNameError::Empty);
    }

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::{ToolNameError, normalize_tool_name, normalize_unique_tool_names};

    #[test]
    fn normalizes_case_and_punctuation_to_snake_case() {
        assert_eq!(
            normalize_tool_name("Get--User.Profile", None),
            Ok("get_user_profile".to_owned())
        );
    }

    #[test]
    fn normalizes_camel_case_to_snake_case() {
        assert_eq!(
            normalize_tool_name("GetUser", None),
            Ok("get_user".to_owned())
        );
    }

    #[test]
    fn rejects_unicode_names() {
        assert_eq!(
            normalize_tool_name("caf\u{00e9}", None),
            Err(ToolNameError::NonAscii)
        );
    }

    #[test]
    fn rejects_names_empty_after_normalization() {
        assert_eq!(normalize_tool_name("---", None), Err(ToolNameError::Empty));
    }

    #[test]
    fn normalizes_and_applies_prefix() {
        assert_eq!(
            normalize_tool_name("List Issues", Some("Git-Hub")),
            Ok("git_hub_list_issues".to_owned())
        );
    }

    #[test]
    fn rejects_names_over_the_exposed_length_bound() {
        let name = "a".repeat(65);
        assert_eq!(
            normalize_tool_name(&name, None),
            Err(ToolNameError::TooLong { limit: 64 })
        );
    }

    #[test]
    fn rejects_prefix_and_name_over_the_combined_length_bound() {
        let prefix = "p".repeat(32);
        let name = "n".repeat(32);
        assert_eq!(
            normalize_tool_name(&name, Some(&prefix)),
            Err(ToolNameError::TooLong { limit: 64 })
        );
    }

    #[test]
    fn rejects_collisions_after_normalization() {
        assert_eq!(
            normalize_unique_tool_names(["Get User", "get-user"], None),
            Err(ToolNameError::Duplicate("get_user".to_owned()))
        );
    }
}
