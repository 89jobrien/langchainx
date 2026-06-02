// TODO(#91): No round-trip conformance tests yet. Current impls (in
//   langchainx-llm tools_openai_like.rs, response_format_openai_like.rs)
//   only implement OpenAIFromLangchain (langchain -> openai direction).
//   Add LangchainFromOpenAI impls, then add round-trip conformance tests.
pub trait LangchainIntoOpenAI<T>: Sized {
    fn into_openai(self) -> T;
}

pub trait LangchainFromOpenAI<T>: Sized {
    fn from_openai(openai: T) -> Self;
}

pub trait OpenAiIntoLangchain<T>: Sized {
    fn into_langchain(self) -> T;
}

pub trait OpenAIFromLangchain<T>: Sized {
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

pub trait TryLangchainIntoOpenAI<T>: Sized {
    type Error;

    fn try_into_openai(self) -> Result<T, Self::Error>;
}

pub trait TryLangchainFromOpenAI<T>: Sized {
    type Error;

    fn try_from_openai(openai: T) -> Result<Self, Self::Error>;
}

pub trait TryOpenAiIntoLangchain<T>: Sized {
    type Error;

    fn try_into_langchain(self) -> Result<T, Self::Error>;
}

pub trait TryOpenAiFromLangchain<T>: Sized {
    type Error;

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
