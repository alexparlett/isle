//! Valve's KeyValues text, as Steam's `.vdf` and `.acf` files hold it: quoted keys with a
//! quoted value or a braced block, and `//` comments.

/// A value: a string or a block of keyed values in file order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Str(String),
    Block(Vec<(String, Value)>),
}

impl Value {
    /// The first value under `key`, ignoring case as Steam does.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Block(entries) => entries
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v),
            Value::Str(_) => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            Value::Block(_) => None,
        }
    }

    pub fn entries(&self) -> &[(String, Value)] {
        match self {
            Value::Block(entries) => entries,
            Value::Str(_) => &[],
        }
    }
}

/// The whole text as one block, or `None` when it does not parse.
pub fn parse(text: &str) -> Option<Value> {
    let mut tokens = Tokens { rest: text };
    let entries = block(&mut tokens, true)?;
    Some(Value::Block(entries))
}

enum Token {
    Str(String),
    Open,
    Close,
}

struct Tokens<'a> {
    rest: &'a str,
}

impl Tokens<'_> {
    fn next(&mut self) -> Option<Token> {
        loop {
            self.rest = self.rest.trim_start();
            if let Some(after) = self.rest.strip_prefix("//") {
                self.rest = after.split_once('\n').map_or("", |(_, r)| r);
                continue;
            }
            break;
        }
        let mut chars = self.rest.chars();
        match chars.next()? {
            '{' => {
                self.rest = chars.as_str();
                Some(Token::Open)
            }
            '}' => {
                self.rest = chars.as_str();
                Some(Token::Close)
            }
            '"' => {
                let mut out = String::new();
                let mut escaped = false;
                for (i, c) in chars.as_str().char_indices() {
                    match (escaped, c) {
                        (true, 'n') => out.push('\n'),
                        (true, 't') => out.push('\t'),
                        (true, c) => out.push(c),
                        (false, '\\') => {
                            escaped = true;
                            continue;
                        }
                        (false, '"') => {
                            self.rest = &chars.as_str()[i + 1..];
                            return Some(Token::Str(out));
                        }
                        (false, c) => out.push(c),
                    }
                    escaped = false;
                }
                None
            }
            _ => {
                let end = self
                    .rest
                    .find(|c: char| c.is_whitespace() || c == '{' || c == '}' || c == '"')
                    .unwrap_or(self.rest.len());
                let word = self.rest[..end].to_owned();
                self.rest = &self.rest[end..];
                Some(Token::Str(word))
            }
        }
    }
}

fn block(tokens: &mut Tokens, top: bool) -> Option<Vec<(String, Value)>> {
    let mut entries = Vec::new();
    loop {
        let key = match tokens.next() {
            None if top => return Some(entries),
            None => return None,
            Some(Token::Close) if !top => return Some(entries),
            Some(Token::Str(key)) => key,
            Some(_) => return None,
        };
        let value = match tokens.next()? {
            Token::Str(s) => Value::Str(s),
            Token::Open => Value::Block(block(tokens, false)?),
            Token::Close => return None,
        };
        entries.push((key, value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_blocks_comments_and_escapes() {
        let text = "// header\n\"AppState\"\n{\n\t\"appid\"\t\t\"10\"\n\t\"name\"\t\t\"A \\\"quoted\\\" name\"\n\t\"UserConfig\"\n\t{\n\t\t\"language\"\t\t\"english\"\n\t}\n}\n";
        let v = parse(text).unwrap();
        let app = v.get("appstate").unwrap();
        assert_eq!(app.get("appid").unwrap().as_str(), Some("10"));
        assert_eq!(app.get("name").unwrap().as_str(), Some("A \"quoted\" name"));
        assert_eq!(
            app.get("UserConfig")
                .unwrap()
                .get("language")
                .unwrap()
                .as_str(),
            Some("english")
        );
    }

    #[test]
    fn refuses_an_unclosed_block() {
        assert!(parse("\"a\" { \"b\" \"c\"").is_none());
    }
}
