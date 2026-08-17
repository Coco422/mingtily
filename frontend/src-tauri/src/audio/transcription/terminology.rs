use super::{TranscriptResult, TranscriptionError, TranscriptionProvider};
use crate::sherpa_asr::TerminologyReplacement;
use async_trait::async_trait;
use std::sync::Arc;

pub fn apply_literal_replacements(text: &str, replacements: &[TerminologyReplacement]) -> String {
    if replacements.is_empty() || text.is_empty() {
        return text.to_string();
    }
    let mut ordered = replacements.iter().enumerate().collect::<Vec<_>>();
    ordered.sort_by(|(left_index, left), (right_index, right)| {
        right
            .source
            .len()
            .cmp(&left.source.len())
            .then(left_index.cmp(right_index))
    });

    let mut output = String::with_capacity(text.len());
    let mut position = 0;
    while position < text.len() {
        if let Some((_, replacement)) = ordered
            .iter()
            .find(|(_, replacement)| text[position..].starts_with(&replacement.source))
        {
            output.push_str(&replacement.target);
            position += replacement.source.len();
        } else {
            let character = text[position..]
                .chars()
                .next()
                .expect("position is inside text");
            output.push(character);
            position += character.len_utf8();
        }
    }
    output
}

pub struct TerminologyCorrectionProvider {
    inner: Arc<dyn TranscriptionProvider>,
    replacements: Vec<TerminologyReplacement>,
}

impl TerminologyCorrectionProvider {
    pub fn wrap(
        inner: Arc<dyn TranscriptionProvider>,
        replacements: Vec<TerminologyReplacement>,
    ) -> Arc<dyn TranscriptionProvider> {
        if replacements.is_empty() {
            inner
        } else {
            Arc::new(Self {
                inner,
                replacements,
            })
        }
    }
}

#[async_trait]
impl TranscriptionProvider for TerminologyCorrectionProvider {
    async fn transcribe(
        &self,
        audio: Vec<f32>,
        language: Option<String>,
    ) -> Result<TranscriptResult, TranscriptionError> {
        let mut result = self.inner.transcribe(audio, language).await?;
        result.text = apply_literal_replacements(&result.text, &self.replacements);
        Ok(result)
    }

    async fn is_model_loaded(&self) -> bool {
        self.inner.is_model_loaded().await
    }

    async fn get_current_model(&self) -> Option<String> {
        self.inner.get_current_model().await
    }

    fn provider_name(&self) -> &'static str {
        self.inner.provider_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(source: &str, target: &str) -> TerminologyReplacement {
        TerminologyReplacement {
            source: source.to_string(),
            target: target.to_string(),
        }
    }

    #[test]
    fn longest_literal_match_wins() {
        let rules = vec![rule("Open", "开放"), rule("OpenAI", "OpenAI 公司")];
        assert_eq!(
            apply_literal_replacements("OpenAI Open", &rules),
            "OpenAI 公司 开放"
        );
    }

    #[test]
    fn replacement_output_is_not_matched_again() {
        let rules = vec![rule("A", "B"), rule("B", "C")];
        assert_eq!(apply_literal_replacements("AB", &rules), "BC");
    }

    #[test]
    fn matching_is_case_sensitive_and_handles_mixed_text() {
        let rules = vec![rule("meetily", "Mingtily"), rule("明天力", "Mingtily")];
        assert_eq!(
            apply_literal_replacements("Meetily meetily 明天力", &rules),
            "Meetily Mingtily Mingtily"
        );
    }
}
