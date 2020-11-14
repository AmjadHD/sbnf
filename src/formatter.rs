use crate::sbnf::lexer::Token;

pub struct FormatterOptions {
    /// Move global parameters and variable declarations before rules.
    pub reorder: bool,
    pub ensure_trailing_newline: bool,
    pub max_line_length: usize,
}

impl Default for FormatterOptions {
    fn default() -> Self {
        FormatterOptions {
            reorder: true,
            ensure_trailing_newline: true,
            max_line_length: 80,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ElementType {
    Preamble,
    Parameters,
    Variable,
    Rule,
    Other,
}

struct Element<'a> {
    element_type: ElementType,
    tokens: &'a [Token<'a>],
}

pub fn format(options: FormatterOptions, tokens: &[Token<'_>]) -> String {
    let mut elements = get_elements(tokens);

    if options.reorder {
        elements.sort_by_key(|element| element.element_type);
    }

    let mut result = String::new();

    for (index, element) in elements.iter().enumerate() {
        write_element(&options, index == 0, element, &mut result);
    }

    trim_trailing_whitespace(&mut result);

    if options.ensure_trailing_newline {
        result.push('\n');
    }

    result
}

fn get_elements<'a>(tokens: &'a [Token<'a>]) -> Vec<Element<'a>> {
    let mut elements = Vec::new();
    let mut element_start = 0;

    while element_start < tokens.len() {
        let first = match tokens[element_start..]
            .iter()
            .position(|token| !is_trivia(token))
        {
            Some(offset) => element_start + offset,
            None => {
                elements.push(Element {
                    element_type: ElementType::Other,
                    tokens: &tokens[element_start..],
                });
                break;
            }
        };

        if element_start == 0
            && first > 0
            && tokens[..first]
                .iter()
                .any(|token| matches!(token, Token::Comment(_)))
        {
            elements.push(Element {
                element_type: ElementType::Preamble,
                tokens: &tokens[..first],
            });
            element_start = first;
            continue;
        }

        let (element_type, element_end) = match tokens[first] {
            Token::OpenSquareBracket => {
                let end = skip_group(tokens, first).unwrap_or(tokens.len());
                (ElementType::Parameters, end)
            }
            Token::Identifier(_) => parse_declaration(tokens, first),
            _ => (ElementType::Other, skip_to_line_end(tokens, first)),
        };

        let element_end = consume_trailing(tokens, element_end);
        elements.push(Element {
            element_type,
            tokens: &tokens[element_start..element_end],
        });
        element_start = element_end;
    }

    elements
}

fn parse_declaration(
    tokens: &[Token<'_>],
    identifier: usize,
) -> (ElementType, usize) {
    let mut index = skip_trivia(tokens, identifier + 1);

    if matches!(tokens.get(index), Some(Token::OpenSquareBracket)) {
        index = skip_group(tokens, index).unwrap_or(tokens.len());
        index = skip_trivia(tokens, index);
    }

    if matches!(tokens.get(index), Some(Token::OpenCurlyBracket)) {
        index = skip_group(tokens, index).unwrap_or(tokens.len());
        index = skip_trivia(tokens, index);
    }

    match tokens.get(index) {
        Some(Token::Equal) => {
            (ElementType::Variable, parse_variable_value(tokens, index + 1))
        }
        Some(Token::Colon) => {
            (ElementType::Rule, find_rule_end(tokens, index + 1))
        }
        _ => (ElementType::Other, skip_to_line_end(tokens, identifier)),
    }
}

fn parse_variable_value(tokens: &[Token<'_>], start: usize) -> usize {
    let index = skip_trivia(tokens, start);

    match tokens.get(index) {
        Some(Token::Literal(_)) | Some(Token::Regex(_)) => index + 1,
        Some(Token::Identifier(_)) => {
            let after_identifier = skip_trivia(tokens, index + 1);

            if matches!(
                tokens.get(after_identifier),
                Some(Token::OpenSquareBracket)
            ) {
                skip_group(tokens, after_identifier).unwrap_or(tokens.len())
            } else {
                index + 1
            }
        }
        _ => skip_to_line_end(tokens, index),
    }
}

fn find_rule_end(tokens: &[Token<'_>], start: usize) -> usize {
    let mut parentheses: usize = 0;
    let mut square_brackets: usize = 0;
    let mut curly_brackets: usize = 0;

    for (index, token) in tokens.iter().enumerate().skip(start) {
        match *token {
            Token::OpenParenthesis => parentheses += 1,
            Token::CloseParenthesis => {
                parentheses = parentheses.saturating_sub(1);
            }
            Token::OpenSquareBracket => square_brackets += 1,
            Token::CloseSquareBracket => {
                square_brackets = square_brackets.saturating_sub(1);
            }
            Token::OpenCurlyBracket => curly_brackets += 1,
            Token::CloseCurlyBracket => {
                curly_brackets = curly_brackets.saturating_sub(1);
            }
            Token::SemiColon
                if parentheses == 0
                    && square_brackets == 0
                    && curly_brackets == 0 =>
            {
                return index + 1;
            }
            _ => {}
        }
    }

    tokens.len()
}

fn skip_group(tokens: &[Token<'_>], start: usize) -> Option<usize> {
    let mut depth: usize = 0;

    for (index, token) in tokens.iter().enumerate().skip(start) {
        match *token {
            Token::OpenParenthesis
            | Token::OpenSquareBracket
            | Token::OpenCurlyBracket => depth += 1,
            Token::CloseParenthesis
            | Token::CloseSquareBracket
            | Token::CloseCurlyBracket => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index + 1);
                }
            }
            _ => {}
        }
    }

    None
}

fn skip_trivia(tokens: &[Token<'_>], start: usize) -> usize {
    let mut index = start;

    while index < tokens.len() && is_trivia(&tokens[index]) {
        index += 1;
    }

    index
}

fn skip_to_line_end(tokens: &[Token<'_>], start: usize) -> usize {
    match tokens[start..]
        .iter()
        .position(|token| matches!(token, Token::Newline))
    {
        Some(offset) => start + offset,
        None => tokens.len(),
    }
}

fn consume_trailing(tokens: &[Token<'_>], start: usize) -> usize {
    let mut index = start;

    while index < tokens.len()
        && matches!(tokens[index], Token::Whitespace(_) | Token::Comment(_))
    {
        index += 1;
    }

    index
}

fn is_trivia(token: &Token<'_>) -> bool {
    matches!(token, Token::Whitespace(_) | Token::Newline | Token::Comment(_))
}

fn write_element(
    options: &FormatterOptions,
    is_first: bool,
    element: &Element<'_>,
    result: &mut String,
) {
    let first = element.tokens.iter().position(|token| !is_trivia(token));
    let last = element.tokens.iter().rposition(|token| !is_trivia(token));

    let (leading, content, trailing) = match (first, last) {
        (Some(first), Some(last)) => (
            &element.tokens[..first],
            &element.tokens[first..=last],
            &element.tokens[last + 1..],
        ),
        _ => (element.tokens, &[][..], &[][..]),
    };

    write_leading(options, leading, is_first, result);
    write_content(options, content, result);
    write_trailing(options, trailing, result);
}

fn write_leading(
    options: &FormatterOptions,
    tokens: &[Token<'_>],
    is_first: bool,
    result: &mut String,
) {
    let mut index = 0;
    let mut newlines = 0;

    while index < tokens.len() {
        match tokens[index] {
            Token::Whitespace(_) => index += 1,
            Token::Newline => {
                newlines += 1;
                index += 1;
            }
            _ => break,
        }
    }

    if !is_first {
        write_separator(newlines, result);
    }

    loop {
        while index < tokens.len()
            && matches!(tokens[index], Token::Whitespace(_) | Token::Newline)
        {
            index += 1;
        }

        if index >= tokens.len() || !matches!(tokens[index], Token::Comment(_))
        {
            break;
        }

        let (paragraphs, next, blank_after) = collect_comments(tokens, index);

        write_comment_paragraphs(options, 0, &paragraphs, result);

        if blank_after {
            result.push('\n');
        }

        index = next;
    }

    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
}

fn write_trailing(
    options: &FormatterOptions,
    tokens: &[Token<'_>],
    result: &mut String,
) {
    let mut index = 0;

    while index < tokens.len() {
        match tokens[index] {
            Token::Whitespace(_) => index += 1,
            Token::Comment(_) => {
                if result.ends_with('\n') {
                    result.pop();
                }

                if !result.is_empty() && !result.ends_with(' ') {
                    result.push(' ');
                }

                let (paragraphs, next, blank_after) =
                    collect_comments(tokens, index);

                write_comment_paragraphs(options, 0, &paragraphs, result);

                if blank_after {
                    result.push('\n');
                }

                index = next;
            }
            _ => index += 1,
        }
    }
}

fn write_content(
    options: &FormatterOptions,
    tokens: &[Token<'_>],
    result: &mut String,
) {
    let mut index = 0;
    let mut segment_start = 0;
    let mut wrote_content = false;

    while index < tokens.len() {
        if !matches!(tokens[index], Token::Comment(_)) {
            index += 1;
            continue;
        }

        let indentation = if wrote_content { 2 } else { 0 };

        write_tokens(
            options,
            &tokens[segment_start..index],
            indentation,
            2,
            result,
        );

        ensure_newline(result);

        let (paragraphs, next, blank_after) = collect_comments(tokens, index);

        write_comment_paragraphs(options, 2, &paragraphs, result);

        if blank_after {
            result.push('\n');
        }

        wrote_content = true;
        index = next;
        segment_start = index;
    }

    let indentation = if wrote_content { 2 } else { 0 };

    write_tokens(options, &tokens[segment_start..], indentation, 2, result);
}

fn write_tokens(
    options: &FormatterOptions,
    tokens: &[Token<'_>],
    initial_indentation: usize,
    continuation_indentation: usize,
    result: &mut String,
) {
    let width = options.max_line_length.max(initial_indentation + 1);
    let continuation = " ".repeat(continuation_indentation);
    let mut words = Vec::new();
    let mut word = String::new();
    let mut previous = None;

    for token in tokens {
        if token.is_whitespace_or_newline()
            || matches!(token, Token::Comment(_))
        {
            continue;
        }

        let space = previous
            .map(|previous| needs_space(previous, *token))
            .unwrap_or(false);

        if space && !word.is_empty() {
            words.push(word);
            word = String::new();
        }

        word.push_str(&token.to_string());
        previous = Some(*token);
    }

    if !word.is_empty() {
        words.push(word);
    }

    let mut line = " ".repeat(initial_indentation);
    let mut line_has_content = false;

    for word in words {
        if line_has_content {
            if line.len() + 1 + word.len() > width {
                result.push_str(&line);
                result.push('\n');
                line = continuation.clone();
            } else {
                line.push(' ');
            }
        }

        line.push_str(&word);
        line_has_content = true;
    }

    if line_has_content {
        result.push_str(&line);
        result.push('\n');
    }
}

fn needs_space(previous: Token<'_>, current: Token<'_>) -> bool {
    if matches!(
        current,
        Token::Colon | Token::Equal | Token::Pipe | Token::SemiColon
    ) {
        return true;
    }

    if matches!(
        current,
        Token::Comma
            | Token::CloseParenthesis
            | Token::CloseSquareBracket
            | Token::CloseCurlyBracket
            | Token::Asterisk
            | Token::QuestionMark
            | Token::OpenSquareBracket
            | Token::OpenCurlyBracket
    ) {
        return false;
    }

    !matches!(
        previous,
        Token::OpenParenthesis
            | Token::OpenSquareBracket
            | Token::OpenCurlyBracket
            | Token::Tilde
            | Token::Percent
            | Token::ExclamationPoint
    )
}

fn write_separator(newlines: usize, result: &mut String) {
    while result.ends_with([' ', '\t', '\r']) {
        result.pop();
    }

    let mut existing = 0;

    while result.ends_with('\n') {
        result.pop();
        existing += 1;
    }

    let count = existing.max(newlines.clamp(1, 2)).min(2);

    for _ in 0..count {
        result.push('\n');
    }
}

fn ensure_newline(result: &mut String) {
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
}

fn trim_trailing_whitespace(result: &mut String) {
    while result.ends_with([' ', '\t', '\r', '\n']) {
        result.pop();
    }
}

#[derive(Debug)]
struct CommentParagraph<'a> {
    prefix: &'a str,
    indentation: usize,
    content: String,
    completed: bool,
}

impl CommentParagraph<'_> {
    fn is_compatible_prefix(&self, prefix: &str) -> bool {
        if self.completed {
            return false;
        }

        if prefix.chars().count() != self.indentation {
            return false;
        }

        prefix.chars().all(|chr| chr == ' ')
    }
}

fn get_comment_line(line: &str) -> (&str, &str, bool) {
    let mut end = line.len();
    let mut has_content = false;

    for (index, chr) in line.char_indices() {
        if chr.is_alphanumeric() {
            end = index;
            has_content = true;
            break;
        } else if !chr.is_ascii_whitespace() {
            has_content = true;
        }
    }

    (&line[..end], &line[end..], has_content)
}

fn collect_comments<'a>(
    tokens: &'a [Token<'a>],
    start: usize,
) -> (Vec<CommentParagraph<'a>>, usize, bool) {
    let mut paragraphs: Vec<CommentParagraph<'a>> = Vec::new();
    let mut index = start;
    let mut newline_count = 0;

    while index < tokens.len() {
        match tokens[index] {
            Token::Comment(string) => {
                newline_count = 0;

                let (prefix, line, has_content) =
                    get_comment_line(&string[1..]);

                if !has_content {
                    if let Some(paragraph) = paragraphs.last_mut() {
                        paragraph.completed = true;
                    }

                    index += 1;
                    continue;
                }

                if paragraphs.is_empty()
                    || !paragraphs.last().unwrap().is_compatible_prefix(prefix)
                {
                    paragraphs.push(CommentParagraph {
                        prefix,
                        indentation: prefix.chars().count(),
                        content: String::new(),
                        completed: false,
                    });
                }

                let paragraph = paragraphs.last_mut().unwrap();

                if !paragraph.content.is_empty() {
                    paragraph.content.push(' ');
                }
                paragraph.content.push_str(line);

                index += 1;
            }
            Token::Newline => {
                newline_count += 1;
                index += 1;

                if newline_count == 2 {
                    break;
                }
            }
            Token::Whitespace(_) => index += 1,
            _ => break,
        }
    }

    (paragraphs, index, newline_count == 2)
}

fn write_comment_paragraphs(
    options: &FormatterOptions,
    indentation: usize,
    paragraphs: &[CommentParagraph<'_>],
    result: &mut String,
) {
    let mut last_did_complete = false;

    for paragraph in paragraphs {
        if last_did_complete {
            write_indentation(indentation, result);
            result.push_str("#\n");
        }
        last_did_complete = paragraph.completed;

        let mut prefix = paragraph.prefix;
        let mut paragraph_indentation = paragraph.indentation;

        if prefix.starts_with(' ') {
            prefix = &prefix[1..];
            paragraph_indentation = paragraph_indentation.saturating_sub(1);
        }

        let wrap_width = options
            .max_line_length
            .saturating_sub(indentation + "# ".len() + paragraph_indentation)
            .max(1);

        for (index, line) in
            textwrap::wrap_iter(&paragraph.content, wrap_width).enumerate()
        {
            write_indentation(indentation, result);
            result.push_str("# ");

            if index == 0 {
                result.push_str(prefix);
            } else {
                write_indentation(paragraph_indentation, result);
            }

            result.push_str(&line);
            result.push('\n');
        }
    }
}

fn write_indentation(indentation: usize, result: &mut String) {
    for _ in 0..indentation {
        result.push(' ');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sbnf::lexer::lex;

    fn fmt(source: &str) -> String {
        let grammar = lex(source);
        format(FormatterOptions::default(), &grammar.tokens)
    }

    #[test]
    fn format_empty() {
        assert_eq!(fmt(""), "\n");
        assert_eq!(fmt("\n"), "\n");
        assert_eq!(fmt("\n\n"), "\n");
        assert_eq!(fmt(" \n\t"), "\n");
    }

    #[test]
    fn format_comments() {
        assert_eq!(fmt("#foo"), "# foo\n");
        assert_eq!(fmt("# foo\n"), "# foo\n");
        assert_eq!(fmt(" # foo\n"), "# foo\n");
        assert_eq!(
            fmt(" # a long line that should get wrapped around due to the \
                 default max line length being 80 characters."),
            "# a long line that should get wrapped around due to the default \
             max line length\n# being 80 characters.\n"
        );
        assert_eq!(fmt("#foo\n#bar\n"), "# foo bar\n");
        assert_eq!(fmt("#foo\n#\n#bar\n"), "# foo\n#\n# bar\n");
        assert_eq!(fmt("#foo\n#\n#\n#bar\n"), "# foo\n#\n# bar\n");
        assert_eq!(fmt("# foo\n\n# bar\n"), "# foo\n\n# bar\n");
        assert_eq!(
            fmt("# Here's some examples:\n\
                 # * foo bar\n\
                 # * A longer example that should cause the line to wrap at \
                 80 characters onto the next line.\n\
                 #     Further indented paragraph here\n"),
            "# Here's some examples:\n\
             # * foo bar\n\
             # * A longer example that should cause the line to wrap at \
             80 characters onto\n\
             #   the next line.\n\
             #     Further indented paragraph here\n"
        );
        assert_eq!(fmt(";"), ";\n");
    }

    #[test]
    fn format_declarations() {
        assert_eq!(fmt("foo=bar"), "foo = bar\n");
        assert_eq!(fmt("foo:bar;"), "foo : bar ;\n");
        assert_eq!(fmt("foo\n:bar;"), "foo : bar ;\n");
        assert_eq!(fmt("foo[bar]{opts}:baz;"), "foo[bar]{opts} : baz ;\n");
        assert_eq!(fmt("foo = `bar`\n"), "foo = `bar`\n");
        assert_eq!(fmt("[B,A]\nfoo:bar;\n"), "[B, A]\nfoo : bar ;\n");
        assert_eq!(fmt("foo:bar; # trailing\n"), "foo : bar ; # trailing\n");
    }

    #[test]
    fn format_reorder_keeps_preamble() {
        let source = "# header\n\nrule : x ;\n\nA = `a`\n";

        assert_eq!(fmt(source), "# header\n\nA = `a`\nrule : x ;\n");
    }

    #[test]
    fn format_reorder() {
        let source = "b : x ;\nA = `a`\nmain : y ;\n";
        let expected = "A = `a`\nb : x ;\nmain : y ;\n";

        assert_eq!(fmt(source), expected);

        let grammar = lex(source);
        let options =
            FormatterOptions { reorder: false, ..FormatterOptions::default() };

        assert_eq!(
            format(options, &grammar.tokens),
            "b : x ;\nA = `a`\nmain : y ;\n"
        );
    }

    #[test]
    fn format_wrapping() {
        let grammar = lex("main : 'a' 'b' 'c' 'd' 'e' 'f' 'g' 'h' 'i' 'j' ;\n");
        let options = FormatterOptions {
            max_line_length: 20,
            ..FormatterOptions::default()
        };
        let formatted = format(options, &grammar.tokens);

        assert!(formatted.lines().all(|line| line.len() <= 20));
        assert!(formatted.contains("main : 'a' 'b'"));
        assert!(formatted.ends_with("'j' ;\n"));
    }

    #[test]
    fn format_comment_in_rule() {
        assert_eq!(
            fmt("foo : bar # comment\n | baz ;\n"),
            "foo : bar\n  # comment\n  | baz ;\n"
        );
    }

    #[test]
    fn format_crlf() {
        assert_eq!(fmt("foo:bar;\r\n"), "foo : bar ;\n");
    }

    #[test]
    fn format_is_idempotent() {
        let source =
            "# header\n\nrule : value ( 'a' | 'b' )* ;\n\nVALUE = `value`\n";
        let formatted = fmt(source);

        assert_eq!(fmt(&formatted), formatted);
    }

    #[test]
    fn format_does_not_split_tokens() {
        let grammar = lex("foo : `a-very-long-literal-with-hyphens` ;\n");
        let options = FormatterOptions {
            max_line_length: 20,
            ..FormatterOptions::default()
        };
        let formatted = format(options, &grammar.tokens);

        assert!(formatted.contains("`a-very-long-literal-with-hyphens`"));
    }
}
