//! Conversion traits between langchainx schemas and OpenAI-compatible types.
/// Converts a langchainx value into an OpenAI-compatible value.
pub trait LangchainIntoOpenAI<T>: Sized {
    /// Converts this value into `T`.
    fn into_openai(self) -> T;
}

/// Constructs a langchainx value from an OpenAI-compatible value.
pub trait LangchainFromOpenAI<T>: Sized {
    /// Converts `openai` into this type.
    fn from_openai(openai: T) -> Self;
}

/// Converts an OpenAI-compatible value into a langchainx value.
pub trait OpenAiIntoLangchain<T>: Sized {
    /// Converts this value into `T`.
    fn into_langchain(self) -> T;
}

/// Constructs an OpenAI-compatible value from a langchainx value.
pub trait OpenAIFromLangchain<T>: Sized {
    /// Converts `langchain` into this type.
    fn from_langchain(langchain: T) -> Self;
}

impl<T, U> LangchainIntoOpenAI<U> for T
where
    U: OpenAIFromLangchain<T>,
{
    fn into_openai(self) -> U {
        U::from_langchain(self)
    }
}

impl<T, U> OpenAiIntoLangchain<U> for T
where
    U: LangchainFromOpenAI<T>,
{
    fn into_langchain(self) -> U {
        U::from_openai(self)
    }
}

// Try into and from OpenAI

/// Fallibly converts a langchainx value into an OpenAI-compatible value.
pub trait TryLangchainIntoOpenAI<T>: Sized {
    /// Error returned when conversion fails.
    type Error;

    /// Attempts the conversion.
    fn try_into_openai(self) -> Result<T, Self::Error>;
}

/// Fallibly constructs a langchainx value from an OpenAI-compatible value.
pub trait TryLangchainFromOpenAI<T>: Sized {
    /// Error returned when conversion fails.
    type Error;

    /// Attempts the conversion.
    fn try_from_openai(openai: T) -> Result<Self, Self::Error>;
}

/// Fallibly converts an OpenAI-compatible value into a langchainx value.
pub trait TryOpenAiIntoLangchain<T>: Sized {
    /// Error returned when conversion fails.
    type Error;

    /// Attempts the conversion.
    fn try_into_langchain(self) -> Result<T, Self::Error>;
}

/// Fallibly constructs an OpenAI-compatible value from a langchainx value.
pub trait TryOpenAiFromLangchain<T>: Sized {
    /// Error returned when conversion fails.
    type Error;

    /// Attempts the conversion.
    fn try_from_langchain(langchain: T) -> Result<Self, Self::Error>;
}

impl<T, U> TryLangchainIntoOpenAI<U> for T
where
    U: TryOpenAiFromLangchain<T>,
{
    type Error = U::Error;

    fn try_into_openai(self) -> Result<U, U::Error> {
        U::try_from_langchain(self)
    }
}

impl<T, U> TryOpenAiIntoLangchain<U> for T
where
    U: TryLangchainFromOpenAI<T>,
{
    type Error = U::Error;

    fn try_into_langchain(self) -> Result<U, U::Error> {
        U::try_from_openai(self)
    }
}
