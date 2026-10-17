//! scalac's source version (`-source`, dotty's `config/SourceVersion.scala`) and its migration table
//! (`config/MigrationVersion.scala`), with the table's reporting (`report.errorOrMigrationWarning`).
//!
//! The versions are 3.10.0's enum in its order, so that a version 3.8.4 names compares as there, and
//! the tag teq tracks (`TAG`) decides which of them a setting or an import may name and the cells of
//! the table that differ between the tags: 3.8.4 knows neither `3.10` nor `3.11`, and its
//! `WithOperator` errors from `future`, not from `3.10`.

/// A dotty tag whose behaviour teq reproduces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tag {
    V3_8_4,
    V3_10_0,
}

/// The tag teq tracks, the release `scala_release::TRACKED` names (docs/COMPATIBILITY.md): the migration
/// moves that one constant, and a release the table does not know is refused when teq is built.
pub const TAG: Tag = Tag::of(crate::scala_release::TRACKED);

impl Tag {
    const fn of(release: &str) -> Tag {
        const fn eq(a: &str, b: &str) -> bool {
            let (a, b) = (a.as_bytes(), b.as_bytes());
            if a.len() != b.len() {
                return false;
            }
            let mut i = 0;
            while i < a.len() {
                if a[i] != b[i] {
                    return false;
                }
                i += 1;
            }
            true
        }
        if eq(release, "3.8.4") {
            Tag::V3_8_4
        } else if eq(release, "3.10.0") {
            Tag::V3_10_0
        } else {
            panic!("the source version's table knows the releases 3.8.4 and 3.10.0")
        }
    }
}

/// A source version: its ordinal in 3.10.0's `SourceVersion`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceVersion(u8);

/// The versions in dotty's order, by their names (`SourceVersion.values`).
const NAMES: [&str; 28] = [
    "3.0-migration", "3.0", "3.1-migration", "3.1", "3.2-migration", "3.2", "3.3-migration", "3.3", "3.4-migration", "3.4",
    "3.5-migration", "3.5", "3.6-migration", "3.6", "3.7-migration", "3.7", "3.8-migration", "3.8", "3.9-migration", "3.9",
    "3.10-migration", "3.10", "3.11-migration", "3.11", "2.13", "future-migration", "future", "never",
];

impl Default for SourceVersion {
    /// `defaultSourceVersion`: the tracked tag's.
    fn default() -> Self {
        match TAG {
            Tag::V3_8_4 => SourceVersion::V3_8,
            Tag::V3_10_0 => SourceVersion::V3_10,
        }
    }
}

impl SourceVersion {
    pub const V3_0_MIGRATION: SourceVersion = SourceVersion(0);
    pub const V3_0: SourceVersion = SourceVersion(1);
    pub const V3_2: SourceVersion = SourceVersion(5);
    pub const V3_3: SourceVersion = SourceVersion(7);
    pub const V3_4: SourceVersion = SourceVersion(9);
    pub const V3_5: SourceVersion = SourceVersion(11);
    pub const V3_6: SourceVersion = SourceVersion(13);
    pub const V3_7: SourceVersion = SourceVersion(15);
    pub const V3_8: SourceVersion = SourceVersion(17);
    pub const V3_9: SourceVersion = SourceVersion(19);
    pub const V3_10: SourceVersion = SourceVersion(21);
    pub const SCALA2: SourceVersion = SourceVersion(24);
    pub const FUTURE: SourceVersion = SourceVersion(26);
    pub const NEVER: SourceVersion = SourceVersion(27);

    /// Every version 3.10.0 names, in order.
    #[cfg(test)]
    pub fn all() -> impl Iterator<Item = SourceVersion> {
        (0..NAMES.len() as u8).map(SourceVersion)
    }

    pub fn name(self) -> &'static str {
        NAMES[self.0 as usize]
    }

    pub fn ordinal(self) -> u32 {
        self.0 as u32
    }

    /// The version a name denotes in 3.10.0's enum (`SourceVersion.valueOf`).
    fn named(text: &str) -> Option<SourceVersion> {
        NAMES.iter().position(|&n| n == text).map(|i| SourceVersion(i as u8))
    }

    /// Whether the tag's enum has the version: 3.8.4's ends its 3.x at `3.9`.
    pub fn known(self, tag: Tag) -> bool {
        tag == Tag::V3_10_0 || !(SourceVersion(20)..=SourceVersion(23)).contains(&self)
    }

    /// `-source`'s value under the tag, or scalac's refusal: a name its enum lacks or one of
    /// `illegalInSettings` (`2.13`, `3.1-migration`, `never`) is not a valid choice.
    pub fn setting(text: &str, tag: Tag) -> Result<SourceVersion, String> {
        match SourceVersion::named(text) {
            Some(v) if v.known(tag) && v != SourceVersion::SCALA2 && v != SourceVersion(2) && v != SourceVersion::NEVER => Ok(v),
            _ => Err(format!("{} is not a valid choice for -source", text)),
        }
    }

    /// What a selector of a source version import names (`allSourceVersionNames`): a version of the
    /// tag's enum, `never` and `3.1-migration` among them, which the import refuses
    /// (`illegalInImports`); a name the enum lacks is no source version import.
    pub fn import_name(text: &str, tag: Tag) -> Option<SourceVersion> {
        SourceVersion::named(text).filter(|v| v.known(tag))
    }

    /// The refusal of an import of a version `illegalInImports` names, with dotty's candidate: the
    /// version without its `-migration` where the enum has one (`Parsers.mkImport`).
    pub fn illegal_import(self, tag: Tag) -> Option<String> {
        if self != SourceVersion(2) && self != SourceVersion::NEVER {
            return None;
        }
        let name = self.name();
        let base = format!("`{}` is not a valid source version", name);
        let candidate = SourceVersion::import_name(&name.replace("-migration", ""), tag);
        Some(match candidate {
            Some(c) => format!("{}, did you mean language.`{}`?", base, c.name()),
            None => base,
        })
    }

    pub fn is_migrating(self) -> bool {
        self.name().ends_with("-migration")
    }

    /// `stable`: a migration version's own.
    pub fn stable(self) -> SourceVersion {
        if self.is_migrating() { SourceVersion(self.0 + 1) } else { self }
    }

    /// `prevMigrating`: the version itself if a migration one, else the migration version before it.
    pub fn prev_migrating(self) -> SourceVersion {
        let mut v = self;
        while !v.is_migrating() {
            v = SourceVersion(v.0 - 1);
        }
        v
    }

    /// `isAtLeast`, a migration version compared as its stable one.
    pub fn is_at_least(self, v: SourceVersion) -> bool {
        self.stable().0 >= v.0
    }

    pub fn is_scala2(self) -> bool {
        self == SourceVersion::SCALA2
    }

    /// `Feature.migrateTo3`: `given` and `enum` are identifiers (`Scanners.toToken`'s
    /// `handleMigration` over `scala3keywords`), and Scala 2's syntax is read with a warning.
    pub fn migrate_to_3(self) -> bool {
        self == SourceVersion::V3_0_MIGRATION
    }

    /// `enablesNewGivens`: the givens of 3.6's syntax (`given t: (n: Int) => C`, a body after a colon).
    pub fn enables_new_givens(self) -> bool {
        self.is_at_least(SourceVersion::V3_6)
    }
}

/// A row of dotty's migration table: a construct deprecated from `warn_from`, refused from
/// `error_from` (3.10.0's 24 rows; `IdentifierDollars` is 3.10.0's alone). A row whose sites teq
/// does not check yet is listed by tests/source-version/pending.txt's runs of its probe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Migration {
    Scala2to3,
    OverrideValParameter,
    ForComprehensionPatternWithoutCase,
    ForComprehensionUncheckedPathDefs,
    NonLocalReturns,
    AscriptionAfterPattern,
    ExplicitContextBoundArgument,
    AlphanumericInfix,
    RemoveThisQualifier,
    UninitializedVars,
    VarargSpliceAscription,
    WildcardType,
    WithOperator,
    FunctionUnderscore,
    NonNamedArgumentInJavaAnnotation,
    AmbiguousNamedTupleSyntax,
    ImportWildcard,
    ImportRename,
    ParameterEnclosedByParenthesis,
    XmlLiteral,
    GivenSyntax,
    ImplicitParamsWithoutUsing,
    Scala2Implicits,
    IdentifierDollars,
}

/// What `report.errorOrMigrationWarning` gives a row's diagnostic at a version.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Silent,
    Warning,
    MigrationWarning,
    Error,
}

impl Migration {
    #[cfg(test)]
    pub const ALL: [Migration; 24] = [
        Migration::Scala2to3,
        Migration::OverrideValParameter,
        Migration::ForComprehensionPatternWithoutCase,
        Migration::ForComprehensionUncheckedPathDefs,
        Migration::NonLocalReturns,
        Migration::AscriptionAfterPattern,
        Migration::ExplicitContextBoundArgument,
        Migration::AlphanumericInfix,
        Migration::RemoveThisQualifier,
        Migration::UninitializedVars,
        Migration::VarargSpliceAscription,
        Migration::WildcardType,
        Migration::WithOperator,
        Migration::FunctionUnderscore,
        Migration::NonNamedArgumentInJavaAnnotation,
        Migration::AmbiguousNamedTupleSyntax,
        Migration::ImportWildcard,
        Migration::ImportRename,
        Migration::ParameterEnclosedByParenthesis,
        Migration::XmlLiteral,
        Migration::GivenSyntax,
        Migration::ImplicitParamsWithoutUsing,
        Migration::Scala2Implicits,
        Migration::IdentifierDollars,
    ];

    /// `(warnFrom, errorFrom)` under the tag.
    pub fn thresholds(self, tag: Tag) -> (SourceVersion, SourceVersion) {
        use SourceVersion as V;
        match self {
            Migration::Scala2to3 => (V::V3_0, V::V3_0),
            Migration::OverrideValParameter => (V::V3_0, V::FUTURE),
            Migration::ForComprehensionPatternWithoutCase => (V::V3_2, V::V3_4),
            Migration::ForComprehensionUncheckedPathDefs | Migration::NonLocalReturns => (V::V3_2, V::FUTURE),
            Migration::AscriptionAfterPattern => (V::V3_3, V::FUTURE),
            Migration::ExplicitContextBoundArgument => (V::V3_4, V::V3_5),
            Migration::WithOperator => (V::V3_4, if tag == Tag::V3_10_0 { V::V3_10 } else { V::FUTURE }),
            Migration::AlphanumericInfix
            | Migration::RemoveThisQualifier
            | Migration::UninitializedVars
            | Migration::VarargSpliceAscription
            | Migration::WildcardType
            | Migration::FunctionUnderscore => (V::V3_4, V::FUTURE),
            Migration::NonNamedArgumentInJavaAnnotation => (V::V3_6, V::V3_6),
            Migration::AmbiguousNamedTupleSyntax => (V::V3_6, V::FUTURE),
            Migration::ImplicitParamsWithoutUsing => (V::V3_7, V::FUTURE),
            Migration::ImportWildcard
            | Migration::ImportRename
            | Migration::ParameterEnclosedByParenthesis
            | Migration::XmlLiteral
            | Migration::GivenSyntax
            | Migration::Scala2Implicits => (V::FUTURE, V::FUTURE),
            Migration::IdentifierDollars => (V::V3_9, V::NEVER),
        }
    }

    /// `report.errorOrMigrationWarning`: nothing under `2.13`; an error from `errorFrom` but at
    /// exactly `errorFrom.prevMigrating`, which warns as a migration; a warning from `warnFrom`.
    pub fn outcome(self, version: SourceVersion) -> Outcome {
        let (warn_from, error_from) = self.thresholds(TAG);
        if version.is_scala2() {
            Outcome::Silent
        } else if version.is_at_least(error_from) {
            if version != error_from.prev_migrating() { Outcome::Error } else { Outcome::MigrationWarning }
        } else if version.is_at_least(warn_from) {
            Outcome::Warning
        } else {
            Outcome::Silent
        }
    }

    #[cfg(test)]
    pub fn name(self) -> String {
        format!("{:?}", self)
    }
}

/// `Message.rewriteNotice`: what `-rewrite` under a version would patch, a line of its own.
pub fn rewrite_notice(what: &str, version: &str) -> String {
    format!("\n{} can be rewritten automatically under -rewrite -source {}.", what, version)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(name: &str) -> SourceVersion {
        SourceVersion::named(name).unwrap()
    }

    #[test]
    fn settings_and_imports_are_the_tags() {
        for ok in ["3.0-migration", "3.0", "3.1", "3.2-migration", "3.9", "future-migration", "future"] {
            assert_eq!(SourceVersion::setting(ok, Tag::V3_8_4), Ok(v(ok)), "{}", ok);
        }
        for bad in ["3.1-migration", "2.13", "never", "3.10", "3.10-migration", "3.11", "3.12", "4.0"] {
            assert_eq!(SourceVersion::setting(bad, Tag::V3_8_4), Err(format!("{} is not a valid choice for -source", bad)));
        }
        assert_eq!(SourceVersion::setting("3.11-migration", Tag::V3_10_0), Ok(v("3.11-migration")));
        assert_eq!(SourceVersion::import_name("2.13", Tag::V3_8_4), Some(SourceVersion::SCALA2));
        assert_eq!(SourceVersion::import_name("3.10", Tag::V3_8_4), None);
        assert_eq!(v("3.1-migration").illegal_import(Tag::V3_8_4).unwrap(), "`3.1-migration` is not a valid source version, did you mean language.`3.1`?");
        assert_eq!(SourceVersion::NEVER.illegal_import(Tag::V3_8_4).unwrap(), "`never` is not a valid source version, did you mean language.`never`?");
        assert_eq!(v("3.3").illegal_import(Tag::V3_8_4), None);
    }

    #[test]
    fn the_order_and_the_migration_versions() {
        assert!(v("3.4-migration").is_at_least(v("3.4")) && !v("3.3").is_at_least(v("3.4")));
        assert_eq!(v("future").prev_migrating(), v("future-migration"));
        assert_eq!(v("3.0").prev_migrating(), v("3.0-migration"));
        assert_eq!(SourceVersion::NEVER.prev_migrating(), v("future-migration"));
        assert!(v("2.13").is_at_least(v("3.9")) && !v("2.13").is_at_least(v("future")));
        assert!(v("3.6-migration").enables_new_givens() && !v("3.5").enables_new_givens());
    }

    #[test]
    fn the_reporting_algorithm() {
        use Outcome::*;
        let at = |m: Migration, s: &str| m.outcome(v(s));
        assert_eq!([at(Migration::WithOperator, "3.3"), at(Migration::WithOperator, "3.4"), at(Migration::WithOperator, "future-migration"), at(Migration::WithOperator, "future")], [Silent, Warning, MigrationWarning, Error]);
        assert_eq!([at(Migration::NonNamedArgumentInJavaAnnotation, "3.5"), at(Migration::NonNamedArgumentInJavaAnnotation, "3.6-migration"), at(Migration::NonNamedArgumentInJavaAnnotation, "3.6")], [Silent, MigrationWarning, Error]);
        assert_eq!([at(Migration::Scala2to3, "3.0-migration"), at(Migration::Scala2to3, "3.0")], [MigrationWarning, Error]);
        assert_eq!([at(Migration::IdentifierDollars, "3.8"), at(Migration::IdentifierDollars, "3.9"), at(Migration::IdentifierDollars, "future")], [Silent, Warning, Warning]);
        assert_eq!(at(Migration::WithOperator, "2.13"), Silent);
        // 3.10.0's WithOperator errors under `future-migration`, past its `errorFrom`'s migration.
        let (_, e) = Migration::WithOperator.thresholds(Tag::V3_10_0);
        assert!(v("future-migration").is_at_least(e) && v("future-migration") != e.prev_migrating());
    }

    /// The versions a row's thresholds make distinct under the reporting algorithm, which
    /// tests/source-version/runs.txt runs each row's probe at: the latest legal setting before
    /// `warnFrom`, `warnFrom`, `errorFrom.prevMigrating` and `errorFrom`, each where it is legal and
    /// gives an outcome of its own.
    fn boundaries(m: Migration, tag: Tag, below: bool) -> Vec<SourceVersion> {
        let (warn_from, error_from) = m.thresholds(tag);
        let legal = |x: SourceVersion| SourceVersion::setting(x.name(), tag).is_ok();
        let mut out = Vec::new();
        if let Some(below) = SourceVersion::all().filter(|&x| below && legal(x) && !x.is_migrating() && x.stable() < warn_from).last() {
            out.push(below);
        }
        if warn_from != error_from && legal(warn_from) {
            out.push(warn_from);
        }
        if error_from != SourceVersion::NEVER {
            for x in [error_from.prev_migrating(), error_from] {
                if legal(x) && !out.contains(&x) {
                    out.push(x);
                }
            }
        }
        out
    }

    #[test]
    fn the_runs_cover_each_rows_boundaries() {
        let runs = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/source-version/runs.txt")).unwrap();
        let names: Vec<&str> = runs.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).collect();
        for m in Migration::ALL {
            let mut want = boundaries(m, TAG, true);
            // A threshold only the next tag can show: its versions, which 3.8.4 refuses.
            if TAG == Tag::V3_8_4 {
                for x in boundaries(m, Tag::V3_10_0, false) {
                    if !x.known(Tag::V3_8_4) && !want.contains(&x) {
                        want.push(x);
                    }
                }
            }
            for x in want {
                let line = format!("{} -source:{}", m.name(), x.name());
                assert!(names.iter().any(|l| l.split("=>").next().unwrap().trim() == line), "runs.txt lacks `{}`", line);
            }
            assert!(names.contains(&m.name().as_str()), "runs.txt lacks the default run of {}", m.name());
        }
    }
}
