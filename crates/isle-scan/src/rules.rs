//! Cleanup's rules (D89; `p19b-cleaner.md` choices 11 and 12): read from KDL, the shipped
//! file with the user's beside it, and matched against the scanned tree in memory.

use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fmt,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use crate::{
    kinds::Kind,
    tree::{Id, Tree},
};

pub const SHIPPED: &str = include_str!("../../../data/cleaner/rules.kdl");

#[derive(Clone, Debug, PartialEq)]
pub struct RulesError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl fmt::Display for RulesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Start {
    Home,
    Steam,
    Anywhere,
}

#[derive(Clone, Debug, PartialEq)]
struct Pattern {
    start: Start,
    parts: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub id: String,
    name: String,
    pub kind: Kind,
    pub safe: bool,
    why: String,
    general: bool,
    files: bool,
    older_than_days: Option<i64>,
    numeric: bool,
    paths: Vec<Pattern>,
    beside: Vec<String>,
    except: Vec<String>,
    unless: Vec<String>,
    endings: Vec<String>,
    names: HashMap<String, String>,
    replace: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Rules {
    pub rules: Vec<Rule>,
    disabled: Vec<String>,
}

/// Where the rules' starts are.
pub struct Context {
    pub home: PathBuf,
    /// Steam's library folders, other disks' included.
    pub libraries: Vec<PathBuf>,
    /// Seconds since the epoch.
    pub now: i64,
}

impl Context {
    pub fn here(libraries: Vec<PathBuf>) -> Self {
        Self {
            home: std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default(),
            libraries,
            now: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs() as i64),
        }
    }
}

/// A rule's find: one row of Cleanup.
#[derive(Clone, Debug, PartialEq)]
pub struct Find {
    pub rule: String,
    /// Named for the app or project that made it.
    pub name: String,
    pub path: PathBuf,
    /// The folder's node; none for a file.
    pub node: Option<Id>,
    pub bytes: u64,
    pub kind: Kind,
    pub safe: bool,
    pub why: String,
}

fn kind_named(name: &str) -> Option<Kind> {
    Some(match name {
        "games" => Kind::Games,
        "video" => Kind::Video,
        "images" => Kind::Images,
        "audio" => Kind::Audio,
        "documents" => Kind::Documents,
        "archives" => Kind::Archives,
        "developer" => Kind::Developer,
        "system" => Kind::System,
        "caches" => Kind::Caches,
        "other" => Kind::Other,
        _ => return None,
    })
}

/// `*` matches any run of bytes, the only wildcard.
fn glob(pattern: &[u8], name: &[u8]) -> bool {
    match pattern.iter().position(|b| *b == b'*') {
        None => pattern == name,
        Some(star) => {
            let (head, tail) = (&pattern[..star], &pattern[star + 1..]);
            name.starts_with(head) && (head.len()..=name.len()).any(|i| glob(tail, &name[i..]))
        }
    }
}

fn pattern(text: &str) -> Option<Pattern> {
    let (start, rest) = [
        ("~/", Start::Home),
        ("{steam}/", Start::Steam),
        ("**/", Start::Anywhere),
    ]
    .into_iter()
    .find_map(|(prefix, start)| text.strip_prefix(prefix).map(|r| (start, r)))?;
    let parts: Vec<String> = rest
        .split('/')
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect();
    (!parts.is_empty() && !parts.iter().any(|p| p == "**" || p == ".." || p == "."))
        .then_some(Pattern { start, parts })
}

fn strings(node: &kdl::KdlNode) -> Vec<String> {
    node.entries()
        .iter()
        .filter(|e| e.name().is_none())
        .filter_map(|e| e.value().as_string().map(str::to_owned))
        .collect()
}

impl Rules {
    pub fn shipped() -> Self {
        Self::parse(SHIPPED).expect("the shipped rules.kdl parses")
    }

    pub fn parse(text: &str) -> Result<Self, RulesError> {
        let at = |offset: usize, message: String| {
            let before = &text[..offset.min(text.len())];
            RulesError {
                line: before.matches('\n').count() + 1,
                column: before
                    .rfind('\n')
                    .map_or(before.len(), |n| before.len() - n - 1)
                    + 1,
                message,
            }
        };
        let doc: kdl::KdlDocument = text.parse().map_err(|err: kdl::KdlError| {
            let first = err.diagnostics.first();
            at(
                first.map_or(0, |d| d.span.offset()),
                first
                    .and_then(|d| d.message.clone().or_else(|| d.label.clone()))
                    .unwrap_or_else(|| "this is not valid KDL".into()),
            )
        })?;
        let mut rules = Rules::default();
        for node in doc.nodes() {
            let offset = node.span().offset();
            match node.name().value() {
                "version" => match node.entries().first().and_then(|e| e.value().as_integer()) {
                    Some(1) => {}
                    _ => return Err(at(offset, "this Isle reads version 1".into())),
                },
                "disable" => rules.disabled.extend(strings(node)),
                "rule" => rules
                    .rules
                    .push(Self::rule(node).map_err(|m| at(offset, m))?),
                other => return Err(at(offset, format!("unknown node “{other}”"))),
            }
        }
        Ok(rules)
    }

    fn rule(node: &kdl::KdlNode) -> Result<Rule, String> {
        let id = strings(node)
            .into_iter()
            .next()
            .ok_or("a rule needs an id")?;
        let text = |key: &str| node.get(key).and_then(|v| v.as_string()).map(str::to_owned);
        let flag = |key: &str| node.get(key).and_then(|v| v.as_bool()).unwrap_or(false);
        let kind_text = text("kind").unwrap_or_else(|| "other".into());
        let mut rule = Rule {
            name: text("name").unwrap_or_else(|| id.clone()),
            kind: kind_named(&kind_text).ok_or_else(|| format!("unknown kind “{kind_text}”"))?,
            safe: flag("safe"),
            why: text("why").unwrap_or_default(),
            general: flag("general"),
            files: flag("files"),
            older_than_days: node
                .get("older-than")
                .and_then(|v| v.as_integer())
                .map(|d| d as i64),
            numeric: flag("numeric"),
            id,
            paths: Vec::new(),
            beside: Vec::new(),
            except: Vec::new(),
            unless: Vec::new(),
            endings: Vec::new(),
            names: HashMap::new(),
            replace: Vec::new(),
        };
        for child in node.children().map(|c| c.nodes()).unwrap_or_default() {
            let values = strings(child);
            match child.name().value() {
                "path" => {
                    for p in values {
                        rule.paths.push(pattern(&p).ok_or_else(|| {
                            format!("a path starts at ~/, {{steam}}/ or **/: “{p}”")
                        })?);
                    }
                }
                "beside" => rule.beside.extend(values),
                "except" => rule.except.extend(values),
                "unless" => rule.unless.extend(values),
                "endings" => rule
                    .endings
                    .extend(values.into_iter().map(|e| e.to_lowercase())),
                "replace" => match values.as_slice() {
                    [from, to] => rule.replace.push((from.clone(), to.clone())),
                    _ => return Err("replace takes what to find and what to put".into()),
                },
                "names" => {
                    for n in child.children().map(|c| c.nodes()).unwrap_or_default() {
                        if let Some(shown) = strings(n).into_iter().next() {
                            rule.names.insert(n.name().value().to_owned(), shown);
                        }
                    }
                }
                other => return Err(format!("unknown part of a rule “{other}”")),
            }
        }
        if rule.paths.is_empty() {
            return Err(format!("rule “{}” has no path", rule.id));
        }
        Ok(rule)
    }

    /// The user's rules over these: a rule of the same id replaces one here, `disable` drops
    /// one, and the rest are added.
    pub fn with(mut self, user: Rules) -> Self {
        for rule in user.rules {
            match self.rules.iter_mut().find(|r| r.id == rule.id) {
                Some(r) => *r = rule,
                None => self.rules.push(rule),
            }
        }
        self.rules.retain(|r| !user.disabled.contains(&r.id));
        self
    }

    /// The shipped rules with the user's file beside them; a user file that does not parse is
    /// left out, with its error in the log.
    pub fn load() -> Self {
        let dir = std::env::var_os("ISLE_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("XDG_CONFIG_HOME")
                    .map(PathBuf::from)
                    .filter(|p| p.is_absolute())
                    .unwrap_or_else(|| Context::here(Vec::new()).home.join(".config"))
                    .join("isle")
            });
        let path = dir.join("cleaner/rules.kdl");
        let shipped = Self::shipped();
        match std::fs::read_to_string(&path) {
            Ok(text) => match Self::parse(&text) {
                Ok(user) => shipped.with(user),
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "the user's cleaner rules do not parse; the shipped ones stand");
                    shipped
                }
            },
            Err(_) => shipped,
        }
    }

    /// Every rule's finds over `tree`: outer before inner, so a match inside one already found
    /// is dropped, and at one place a named rule's before the general one's.
    pub fn find(&self, tree: &Tree, ctx: &Context) -> Vec<Find> {
        let mut order: Vec<&Rule> = self.rules.iter().filter(|r| !r.general).collect();
        order.extend(self.rules.iter().filter(|r| r.general));
        let mut hits: Vec<(usize, usize, Find)> = Vec::new();
        for (rank, rule) in order.iter().enumerate() {
            for find in rule.finds(tree, ctx) {
                hits.push((find.path.components().count(), rank, find));
            }
        }
        hits.sort_by_key(|h| (h.0, h.1));
        let mut kept: HashSet<PathBuf> = HashSet::new();
        let mut out = Vec::new();
        for (_, _, find) in hits {
            if find.path.ancestors().any(|a| kept.contains(a)) {
                continue;
            }
            kept.insert(find.path.clone());
            out.push(find);
        }
        out
    }
}

/// A place a pattern reached, before the rule's conditions.
struct Hit {
    path: PathBuf,
    node: Option<Id>,
    captures: Vec<String>,
    project: Option<PathBuf>,
    library: Option<PathBuf>,
}

fn last_use(path: &Path) -> Option<(u64, i64)> {
    use rustix::fs::{AtFlags, StatxFlags};
    let st = rustix::fs::statx(
        rustix::fs::CWD,
        path,
        AtFlags::SYMLINK_NOFOLLOW,
        StatxFlags::BLOCKS | StatxFlags::ATIME | StatxFlags::MTIME,
    )
    .ok()?;
    Some((
        st.stx_blocks * 512,
        st.stx_mtime.tv_sec.max(st.stx_atime.tv_sec),
    ))
}

impl Rule {
    fn finds(&self, tree: &Tree, ctx: &Context) -> Vec<Find> {
        let mut hits = Vec::new();
        for pattern in &self.paths {
            match pattern.start {
                Start::Home => {
                    if let Some(id) = tree.find(&ctx.home) {
                        self.walk(tree, id, &pattern.parts, Vec::new(), None, None, &mut hits);
                    }
                }
                Start::Steam => {
                    for lib in &ctx.libraries {
                        if let Some(id) = tree.find(lib) {
                            self.walk(
                                tree,
                                id,
                                &pattern.parts,
                                Vec::new(),
                                None,
                                Some(lib),
                                &mut hits,
                            );
                        }
                    }
                }
                Start::Anywhere => {
                    let first = pattern.parts[0].as_bytes();
                    for id in 1..tree.nodes.len() as Id {
                        if glob(first, tree.name(id).as_bytes()) {
                            let parent = tree.node(id).parent;
                            let project = Some(tree.path(parent));
                            let captures = if first.contains(&b'*') {
                                vec![tree.name(id).to_string_lossy().into_owned()]
                            } else {
                                Vec::new()
                            };
                            self.walk(
                                tree,
                                id,
                                &pattern.parts[1..],
                                captures,
                                project,
                                None,
                                &mut hits,
                            );
                        }
                    }
                }
            }
        }
        hits.into_iter()
            .filter_map(|h| self.judge(tree, ctx, h))
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn walk(
        &self,
        tree: &Tree,
        at: Id,
        parts: &[String],
        captures: Vec<String>,
        project: Option<PathBuf>,
        library: Option<&PathBuf>,
        hits: &mut Vec<Hit>,
    ) {
        let Some((part, rest)) = parts.split_first() else {
            hits.push(Hit {
                path: tree.path(at),
                node: Some(at),
                captures,
                project,
                library: library.cloned(),
            });
            return;
        };
        let wild = part.contains('*');
        for child in tree.children(at) {
            let name = tree.name(child);
            if glob(part.as_bytes(), name.as_bytes()) {
                let mut c = captures.clone();
                if wild {
                    c.push(name.to_string_lossy().into_owned());
                }
                self.walk(tree, child, rest, c, project.clone(), library, hits);
            }
        }
        // The tree keeps folders only, so a rule that takes files reads its last folder.
        if rest.is_empty() && self.files {
            let folder = tree.path(at);
            for entry in std::fs::read_dir(&folder).into_iter().flatten().flatten() {
                let is_file = entry.file_type().is_ok_and(|t| !t.is_dir());
                let name = entry.file_name();
                if is_file && glob(part.as_bytes(), name.as_bytes()) {
                    let mut c = captures.clone();
                    if wild {
                        c.push(name.to_string_lossy().into_owned());
                    }
                    hits.push(Hit {
                        path: entry.path(),
                        node: None,
                        captures: c,
                        project: project.clone(),
                        library: library.cloned(),
                    });
                }
            }
        }
    }

    /// The rule's conditions on one hit, and the row it makes.
    fn judge(&self, tree: &Tree, ctx: &Context, hit: Hit) -> Option<Find> {
        if hit.path.components().any(|c| c.as_os_str() == ".git") {
            return None;
        }
        let name = hit.path.file_name()?.to_string_lossy().into_owned();
        if self
            .except
            .iter()
            .any(|e| glob(e.as_bytes(), name.as_bytes()))
        {
            return None;
        }
        if self.numeric
            && !(name.len() <= 9
                && name.bytes().all(|b| b.is_ascii_digit())
                && !name.starts_with('0'))
        {
            return None;
        }
        if !self.endings.is_empty() {
            let lower = name.to_lowercase();
            if hit.node.is_some() || !self.endings.iter().any(|e| lower.ends_with(e.as_str())) {
                return None;
            }
        }
        let parent = hit.path.parent()?;
        let beside = if self.beside.is_empty() {
            None
        } else {
            Some(
                self.beside
                    .iter()
                    .find(|b| parent.join(b).exists())?
                    .clone(),
            )
        };
        let fill = |t: &str| {
            t.replace("{name}", &name)
                .replace("{match}", &hit.path.to_string_lossy())
                .replace(
                    "{library}",
                    &hit.library
                        .as_deref()
                        .unwrap_or(Path::new(""))
                        .to_string_lossy(),
                )
        };
        if self.unless.iter().any(|u| Path::new(&fill(u)).exists()) {
            return None;
        }
        let (bytes, used) = match hit.node {
            Some(id) => (tree.node(id).bytes, None),
            None => last_use(&hit.path).map(|(b, u)| (b, Some(u)))?,
        };
        if let Some(days) = self.older_than_days {
            let used = used.or_else(|| last_use(&hit.path).map(|(_, u)| u))?;
            if used > ctx.now - days * 24 * 3600 {
                return None;
            }
        }
        if bytes == 0 {
            return None;
        }
        let captures: Vec<String> = hit
            .captures
            .iter()
            .map(|c| {
                self.replace
                    .iter()
                    .fold(c.clone(), |s, (from, to)| s.replace(from.as_str(), to))
            })
            .collect();
        let shown = self
            .names
            .get(&name)
            .cloned()
            .unwrap_or_else(|| name.clone());
        let project = hit
            .project
            .as_deref()
            .and_then(Path::file_name)
            .map(OsStr::to_string_lossy)
            .unwrap_or_default();
        let mut label = self
            .name
            .replace("{name}", &shown)
            .replace("{project}", &project);
        for (i, c) in captures.iter().enumerate() {
            label = label.replace(&format!("{{{}}}", i + 1), c);
        }
        Some(Find {
            rule: self.id.clone(),
            name: label,
            path: hit.path,
            node: hit.node,
            bytes,
            kind: self.kind,
            safe: self.safe,
            why: self
                .why
                .replace("{beside}", beside.as_deref().unwrap_or("")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs() {
        assert!(glob(b"*", b"anything"));
        assert!(glob(b"cmake-build-*", b"cmake-build-debug"));
        assert!(!glob(b"cmake-build-*", b"build"));
        assert!(glob(b"*.lock", b"x.lock"));
        assert!(glob(b"models--*", b"models--a--b"));
        assert!(glob(b"a*b*c", b"aXbYc"));
        assert!(!glob(b"a*b*c", b"aXbY"));
    }

    #[test]
    fn the_shipped_rules_parse() {
        let rules = Rules::shipped();
        assert!(rules.rules.len() > 30);
        assert_eq!(rules.rules.iter().filter(|r| r.general).count(), 1);
        let mut ids = HashSet::new();
        assert!(
            rules.rules.iter().all(|r| ids.insert(r.id.clone())),
            "ids are unique"
        );
    }

    #[test]
    fn the_users_rules_replace_disable_and_add() {
        let user = Rules::parse(
            r#"version 1
            disable "npm"
            rule "cargo" name="Cargo, mine" kind="developer" { path "~/.cargo/registry" }
            rule "mine" name="Renders" kind="video" { path "~/Renders" }"#,
        )
        .unwrap();
        let rules = Rules::shipped().with(user);
        assert!(!rules.rules.iter().any(|r| r.id == "npm"));
        assert_eq!(
            rules.rules.iter().find(|r| r.id == "cargo").unwrap().name,
            "Cargo, mine"
        );
        assert!(rules.rules.iter().any(|r| r.id == "mine"));
    }

    #[test]
    fn a_bad_file_says_where() {
        let err =
            Rules::parse("version 1\nrule \"x\" kind=\"socks\" { path \"~/x\" }").unwrap_err();
        assert_eq!(
            (err.line, err.message.as_str()),
            (2, "unknown kind “socks”")
        );
        assert!(
            Rules::parse("version 1\nrule \"x\" { path \"/etc\" }").is_err(),
            "a path from the root is refused"
        );
        assert!(Rules::parse("version 2").is_err());
    }
}
