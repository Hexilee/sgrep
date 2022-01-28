use colored::Colorize;
use tantivy::SnippetGenerator;

pub fn highlight(generator: &SnippetGenerator, text: &str) -> Option<String> {
    let snippet = generator.snippet(text);
    if snippet.fragments().is_empty() {
        return None;
    }

    let offset = match text.find(snippet.fragments()) {
        Some(i) => i,
        None => return None,
    };

    let mut result = String::with_capacity(text.len());
    result.push_str(&text[0..offset]);
    let mut start_from = 0;

    for fragment_range in snippet.highlighted() {
        result.push_str(&snippet.fragments()[start_from..fragment_range.start]);
        result.push_str(&format!(
            "{}",
            &snippet.fragments()[fragment_range.clone()].red().bold()
        ));
        start_from = fragment_range.end;
    }

    result.push_str(&text[start_from + offset..]);
    Some(result)
}
