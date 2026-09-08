//! Round-trip assertions for provider schema conversions.

use langchainx_core::schemas::convert::{
    LangchainIntoOpenAI, OpenAiIntoLangchain, TryLangchainIntoOpenAI, TryOpenAiIntoLangchain,
};
use std::fmt::Debug;

/// Asserts that an infallible langchainx-to-OpenAI conversion preserves the original value.
pub fn assert_openai_round_trip<L, O>(value: L)
where
    L: Clone + Debug + PartialEq + LangchainIntoOpenAI<O>,
    O: OpenAiIntoLangchain<L>,
{
    let round_trip = value.clone().into_openai().into_langchain();
    assert_eq!(round_trip, value, "OpenAI conversion must round-trip");
}

/// Asserts that a fallible langchainx-to-OpenAI conversion preserves the original value.
pub fn assert_openai_try_round_trip<L, O>(value: L)
where
    L: Clone + Debug + PartialEq + TryLangchainIntoOpenAI<O>,
    <L as TryLangchainIntoOpenAI<O>>::Error: Debug,
    O: TryOpenAiIntoLangchain<L>,
    <O as TryOpenAiIntoLangchain<L>>::Error: Debug,
{
    let openai = value
        .clone()
        .try_into_openai()
        .expect("langchainx value must convert to OpenAI");
    let round_trip = openai
        .try_into_langchain()
        .expect("OpenAI value must convert back to langchainx");
    assert_eq!(round_trip, value, "OpenAI conversion must round-trip");
}
