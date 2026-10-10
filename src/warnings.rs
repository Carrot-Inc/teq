//! The reporting policy of warnings, scalac 3.8.4's: a warning carries its identity (dotty's
//! message id, its category, its origin) and is kept with the diagnostics until the build is
//! over; only then is it filtered by the `@nowarn` annotations of its source and by `-Wconf`, a
//! conditional warning whose flag is off counted into a summary, and the rest reported, counted
//! and made fatal by `--werror` (`Reporter.issueIfNotSuppressed`, `Run.suppressions`,
//! `Reporter.summarizeUnreportedWarnings`). `Diagnostics::report` (`source.rs`) runs the policy,
//! the one place every presentation goes through: the command line's, a watch session's, the
//! language server's, the interpreter's.
//!
//! **Suppressions** (`Run.suppressions.registerNowarn`): each `@nowarn` of a program file, on a
//! definition (its whole range, annotations included) or an ascription (`e: @nowarn`), with its
//! filters read as `-Wconf`'s (`""` any, `"none"` none, `"v"` verbose). A warning in the range
//! that every filter matches is silenced, the first matching annotation marked used and the
//! others superseded; under `--wunused nowarn` an annotation nothing used is reported. They are
//! read of the trees of the files at the report, so that they do not depend on the order the
//! workers typed in, and a watch session's edit replaces them with its file.
//!
//! **`-Wconf`** (`WConf.fromSettings`): the rules of every `--wconf`, the rightmost first; a
//! warning takes the action of the first whose filters all match it. A configuration that does
//! not parse is reported as a warning before the first warning and filters nothing.

use crate::interp::regex::{unit_chars, Regex};
use crate::source::{FileId, Span};

/// What kind of warning a diagnostic is, as `-Wconf`'s `cat=` and the conditional warnings read
/// it (`Diagnostic.scala`'s classes).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Category {
    #[default]
    Plain,
    /// `DeprecationWarning`: summarized unless `--deprecation`.
    Deprecation,
    /// `FeatureWarning`: summarized unless `--feature`.
    Feature,
    /// `UncheckedWarning`: on by default (scalac's `-unchecked`).
    Unchecked,
    Configuration,
}

impl Category {
    /// The option whose absence summarizes the category, as scalac's message names it.
    fn enabling_option(self) -> Option<&'static str> {
        match self {
            Category::Deprecation => Some("--deprecation"),
            Category::Feature => Some("--feature"),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Category::Plain => "",
            Category::Deprecation => "deprecation",
            Category::Feature => "feature",
            Category::Unchecked => "unchecked",
            Category::Configuration => "configuration",
        }
    }
}

/// The ids of the messages teq gives with scalac's (`ErrorMessageID.errorNumber`).
pub mod id {
    pub const EMPTY_CATCH_AND_FINALLY_BLOCK: u16 = 2;
    pub const PATTERN_MATCH_EXHAUSTIVITY: u16 = 29;
    pub const MATCH_CASE_UNREACHABLE: u16 = 30;
    pub const UNCHECKED_TYPE_PATTERN: u16 = 92;
    pub const PURE_EXPRESSION_IN_STATEMENT_POSITION: u16 = 129;
    pub const VALUE_DISCARDING: u16 = 175;
    pub const UNUSED_SYMBOL: u16 = 198;
    pub const FORMAT_INTERPOLATION_ERROR: u16 = 209;
    pub const MATCH_IS_NOT_PARTIAL_FUNCTION: u16 = 211;
}

/// The names of dotty 3.8.4's message ids by number from 0 (`ErrorMessageID`, in its order), an
/// inactive one with a `-` in front: what `id=` and `name=` read and verbose help prints.
const ID_NAMES: [&str; 228] = [
    "-EmptyCatchOrFinallyBlockID", "EmptyCatchBlockID", "EmptyCatchAndFinallyBlockID", "DeprecatedWithOperatorID",
    "CaseClassMissingParamListID", "DuplicateBindID", "MissingIdentID", "TypeMismatchID", "NotAMemberID",
    "EarlyDefinitionsNotSupportedID", "-TopLevelImplicitClassID", "ImplicitCaseClassID", "ImplicitClassPrimaryConstructorArityID",
    "ObjectMayNotHaveSelfTypeID", "-TupleTooLongID", "RepeatedModifierID", "InterpolatedStringErrorID", "UnboundPlaceholderParameterID",
    "IllegalStartSimpleExprID", "MissingReturnTypeID", "YieldOrDoExpectedInForComprehensionID", "ProperDefinitionNotFoundID",
    "ByNameParameterNotSupportedID", "WrongNumberOfTypeArgsID", "IllegalVariableInPatternAlternativeID", "IdentifierExpectedID",
    "AuxConstructorNeedsNonImplicitParameterID", "VarArgsParamMustComeLastID", "IllegalLiteralID", "PatternMatchExhaustivityID",
    "MatchCaseUnreachableID", "SeqWildcardPatternPosID", "IllegalStartOfSimplePatternID", "PkgDuplicateSymbolID",
    "ExistentialTypesNoLongerSupportedID", "UnboundWildcardTypeID", "-DanglingThisInPathID", "OverridesNothingID",
    "OverridesNothingButNameExistsID", "ForwardReferenceExtendsOverDefinitionID", "ExpectedTokenButFoundID",
    "MixedLeftAndRightAssociativeOpsID", "CantInstantiateAbstractClassOrTraitID", "UnreducibleApplicationID",
    "OverloadedOrRecursiveMethodNeedsResultTypeID", "RecursiveValueNeedsResultTypeID", "CyclicReferenceInvolvingID",
    "CyclicReferenceInvolvingImplicitID", "SuperQualMustBeParentID", "AmbiguousReferenceID", "MethodDoesNotTakeParametersId",
    "AmbiguousOverloadID", "ReassignmentToValID", "TypeDoesNotTakeParametersID", "-ParameterizedTypeLacksArgumentsID",
    "VarValParametersMayNotBeCallByNameID", "MissingTypeParameterForID", "DoesNotConformToBoundID", "DoesNotConformToSelfTypeID",
    "DoesNotConformToSelfTypeCantBeInstantiatedID", "AbstractMemberMayNotHaveModifierID", "-TopLevelCantBeImplicitID",
    "TypesAndTraitsCantBeImplicitID", "OnlyClassesCanBeAbstractID", "AbstractOverrideOnlyInTraitsID", "TraitsMayNotBeFinalID",
    "NativeMembersMayNotHaveImplementationID", "OnlyClassesCanHaveDeclaredButUndefinedMembersID", "CannotExtendAnyValID",
    "CannotHaveSameNameAsID", "ValueClassesMayNotDefineInnerID", "ValueClassesMayNotDefineNonParameterFieldID",
    "ValueClassesMayNotDefineASecondaryConstructorID", "ValueClassesMayNotContainInitalizationID",
    "ValueClassesMayNotBeAbstractID", "ValueClassesMayNotBeContaintedID", "ValueClassesMayNotWrapAnotherValueClassID",
    "ValueClassParameterMayNotBeAVarID", "ValueClassNeedsExactlyOneValParamID", "-OnlyCaseClassOrCaseObjectAllowedID",
    "-ExpectedTopLevelDefID", "AnonymousFunctionMissingParamTypeID", "SuperCallsNotAllowedInlineableID",
    "NotAPathID", "WildcardOnTypeArgumentNotAllowedOnNewID", "-FunctionTypeNeedsNonEmptyParameterListID",
    "WrongNumberOfParametersID", "DuplicatePrivateProtectedQualifierID", "ExpectedStartOfTopLevelDefinitionID",
    "MissingReturnTypeWithReturnStatementID", "NoReturnFromInlineableID", "ReturnOutsideMethodDefinitionID",
    "UncheckedTypePatternID", "ExtendFinalClassID", "-EnumCaseDefinitionInNonEnumOwnerID", "ExpectedTypeBoundOrEqualsID",
    "ClassAndCompanionNameClashID", "TailrecNotApplicableID", "FailureToEliminateExistentialID",
    "OnlyFunctionsCanBeFollowedByUnderscoreID", "MissingEmptyArgumentListID", "DuplicateNamedTypeParameterID",
    "UndefinedNamedTypeParameterID", "IllegalStartOfStatementID", "TraitIsExpectedID", "-TraitRedefinedFinalMethodFromAnyRefID",
    "PackageNameAlreadyDefinedID", "UnapplyInvalidNumberOfArgumentsID", "UnapplyInvalidReturnTypeID",
    "StaticFieldsOnlyAllowedInObjectsID", "CyclicInheritanceID", "BadSymbolicReferenceID", "UnableToExtendSealedClassID",
    "SymbolHasUnparsableVersionNumberID", "SymbolChangedSemanticsInVersionID", "UnableToEmitSwitchID", "MissingCompanionForStaticID",
    "PolymorphicMethodMissingTypeInParentID", "ParamsNoInlineID", "JavaSymbolIsNotAValueID", "DoubleDefinitionID",
    "MatchCaseOnlyNullWarningID", "ImportedTwiceID", "TypeTestAlwaysDivergesID", "TermMemberNeedsNeedsResultTypeForImplicitSearchID",
    "ClassCannotExtendEnumID", "ValueClassParameterMayNotBeCallByNameID", "NotAnExtractorID", "MemberWithSameNameAsStaticID",
    "PureExpressionInStatementPositionID", "TraitCompanionWithMutableStaticID", "LazyStaticFieldID", "StaticOverridingNonStaticMembersID",
    "OverloadInRefinementID", "NoMatchingOverloadID", "StableIdentPatternID", "StaticFieldsShouldPrecedeNonStaticID",
    "IllegalSuperAccessorID", "TraitParameterUsedAsParentPrefixID", "UnknownNamedEnclosingClassOrObjectID",
    "IllegalCyclicTypeReferenceID", "MissingTypeParameterInTypeAppID", "-SkolemInInferredID", "ErasedTypesCanOnlyBeFunctionTypesID",
    "CaseClassMissingNonImplicitParamListID", "EnumerationsShouldNotBeEmptyID", "IllegalParameterInitID",
    "RedundantModifierID", "TypedCaseDoesNotExplicitlyExtendTypedEnumID", "IllegalRedefinitionOfStandardKindID",
    "-NoExtensionMethodAllowedID", "-ExtensionMethodCannotHaveTypeParamsID", "-ExtensionCanOnlyHaveDefsID",
    "UnexpectedPatternForSummonFromID", "AnonymousInstanceCannotBeEmptyID", "-TypeSpliceInValPatternID",
    "ModifierNotAllowedForDefinitionID", "CannotExtendJavaEnumID", "InvalidReferenceInImplicitNotFoundAnnotationID",
    "TraitMayNotDefineNativeMethodID", "JavaEnumParentArgsID", "AlreadyDefinedID", "CaseClassInInlinedCodeID",
    "-OverrideTypeMismatchErrorID", "OverrideErrorID", "MatchableWarningID", "CannotExtendFunctionID",
    "LossyWideningConstantConversionID", "ImplicitSearchTooLargeID", "TargetNameOnTopLevelClassID", "NotClassTypeID",
    "MissingArgumentID", "MissingImplicitArgumentID", "CannotBeAccessedID", "InlineGivenShouldNotBeFunctionID",
    "ValueDiscardingID", "UnusedNonUnitValueID", "ConstrProxyShadowsID", "MissingArgumentListID", "MatchTypeScrutineeCannotBeHigherKindedID",
    "AmbiguousExtensionMethodID", "UnqualifiedCallToAnyRefMethodID", "NotConstantID", "ClosureCannotHaveInternalParameterDependenciesID",
    "MatchTypeNoCasesID", "UnimportedAndImportedID", "ImplausiblePatternWarningID", "SynchronizedCallOnBoxedClassID",
    "VarArgsParamCannotBeGivenID", "ExtractorNotFoundID", "PureUnitExpressionID", "MatchTypeLegacyPatternID",
    "UnstableInlineAccessorID", "VolatileOnValID", "ExtensionNullifiedByMemberID", "PhantomSymbolNotValueID",
    "-ContextBoundCompanionNotValueID", "InlinedAnonClassWarningID", "UnusedSymbolID", "TailrecNestedCallID", "FinalLocalDefID",
    "NonNamedArgumentInJavaAnnotationID", "QuotedTypeMissingID", "DeprecatedAssignmentSyntaxID",
    "DeprecatedInfixNamedArgumentSyntaxID", "GivenSearchPriorityID", "EnumMayNotBeValueClassesID", "IllegalUnrollPlacementID",
    "ExtensionHasDefaultID", "FormatInterpolationErrorID", "ValueClassCannotExtendAliasOfAnyValID", "MatchIsNotPartialFunctionID",
    "OnlyFullyDependentAppliedConstructorTypeID", "PointlessAppliedConstructorTypeID", "IllegalContextBoundsID",
    "NamedPatternNotApplicableID", "UnnecessaryNN", "ErasedNotPureID", "IllegalErasedDefID", "CannotInstantiateQuotedTypeVarID",
    "DefaultShadowsGivenID", "RecurseWithDefaultID", "EncodedPackageNameID", "CannotBeIncludedID", "OverrideClassID",
    "InferUnionWarningID", "TypeParameterShadowsTypeID", "PrivateShadowsTypeID",
];

/// No message id: a message of teq's own, or one scalac gives none.
pub const NO_ID: u16 = u16::MAX;

/// The number of the message named `name` (`ErrorMessageID.valueOf(name + "ID")`).
fn id_named(name: &str) -> Option<u16> {
    ID_NAMES.iter().position(|n| n.trim_start_matches('-').strip_suffix("ID") == Some(name)).map(|i| i as u16)
}

/// What a filter of `-Wconf` or of `@nowarn` matches (`MessageFilter`).
#[derive(Clone)]
pub enum Filter {
    Any,
    None,
    Category(Category),
    Id(u16),
    Message(Pattern),
    Source(Pattern),
    Origin(Pattern),
}

/// A regular expression of a filter, as Java's `findFirstIn` reads it: anywhere in the text.
#[derive(Clone)]
pub struct Pattern(std::sync::Arc<Regex>);

impl Pattern {
    fn compile(source: &str) -> Result<Pattern, String> {
        Regex::compile(source).map(|r| Pattern(std::sync::Arc::new(r))).map_err(|e| {
            // Java's `PatternSyntaxException.getMessage`: the message and the pattern, then a
            // caret under the index where it is inside the pattern.
            let index = e.split_once(" near index ").and_then(|(_, rest)| rest.split('\n').next()).and_then(|i| i.parse::<usize>().ok());
            let mut message = e;
            if let Some(i) = index.filter(|&i| i < source.chars().count()) {
                message.push('\n');
                message.push_str(&" ".repeat(i));
                message.push('^');
            }
            format!("invalid pattern `{}`: {}", source, message)
        })
    }

    pub fn finds_in(&self, text: &str) -> bool {
        let (chars, _) = unit_chars(text);
        matches!(self.0.find(&chars, 0, false, false), Ok(Some(_)))
    }
}

/// What a diagnostic is to a filter: its message, its id, category and origin, and the path of
/// its source.
pub struct Subject<'a> {
    pub msg: &'a str,
    pub id: u16,
    pub category: Category,
    pub origin: Option<&'a str>,
    pub path: Option<&'a str>,
}

impl Filter {
    fn matches(&self, s: &Subject) -> bool {
        match self {
            Filter::Any => true,
            Filter::None => false,
            Filter::Category(c) => s.category == *c,
            Filter::Id(n) => s.id == *n,
            Filter::Message(p) => p.finds_in(s.msg),
            Filter::Source(p) => s.path.is_some_and(|path| p.finds_in(path)),
            // A feature warning's origin is its use site, which no filter reads.
            Filter::Origin(p) => s.category != Category::Feature && s.origin.is_some_and(|o| p.finds_in(o)),
        }
    }

    /// One filter of `-Wconf`'s grammar (`WConf.parseFilter`).
    fn parse(s: &str) -> Result<Filter, String> {
        if s == "any" {
            return Ok(Filter::Any);
        }
        let Some((filter, conf)) = s.split_once('=').filter(|(f, c)| !f.is_empty() && !c.is_empty()) else {
            return Err(format!("unknown filter: {}", s));
        };
        match filter {
            "msg" => Pattern::compile(conf).map(Filter::Message),
            "id" => {
                let digits = conf.strip_prefix('E').unwrap_or(conf);
                if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(format!("invalid error message id: {}", conf));
                }
                let n: Option<usize> = digits.parse().ok();
                match n.and_then(|n| ID_NAMES.get(n).map(|name| (n, name))) {
                    Some((n, name)) if !name.starts_with('-') => Ok(Filter::Id(n as u16)),
                    Some(_) => Err(format!("E{} is marked as inactive.", digits)),
                    None => Err(format!("Unknown error message number: E{}", digits)),
                }
            }
            "name" => id_named(conf).map(Filter::Id).ok_or_else(|| format!("unknown error message name: {}", conf)),
            "cat" => match conf {
                "configuration" => Ok(Filter::Category(Category::Configuration)),
                "deprecation" => Ok(Filter::Category(Category::Deprecation)),
                "feature" => Ok(Filter::Category(Category::Feature)),
                "unchecked" => Ok(Filter::Category(Category::Unchecked)),
                _ => Err(format!("unknown category: {}", conf)),
            },
            "src" => Pattern::compile(conf).map(Filter::Source),
            "origin" => Pattern::compile(conf).map(Filter::Origin),
            _ => Err(format!("unknown filter: {}", filter)),
        }
    }

    /// Filters joined by `&` (`WConf.parseFilters`).
    fn parse_all(s: &str) -> Result<Vec<Filter>, Vec<String>> {
        let mut errors = Vec::new();
        let mut filters = Vec::new();
        for part in s.split('&') {
            match Filter::parse(part) {
                Ok(f) => filters.push(f),
                Err(e) => errors.push(e),
            }
        }
        if !errors.is_empty() {
            Err(errors)
        } else if filters.is_empty() {
            Err(vec!["no filters or no action defined".to_string()])
        } else {
            Ok(filters)
        }
    }
}

/// What `-Wconf` does with a warning (`Action`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Error,
    Warning,
    Verbose,
    Info,
    Silent,
    Default,
}

/// The parsed `-Wconf` rules, the one that wins first.
#[derive(Clone, Default)]
pub struct WConf {
    confs: Vec<(Vec<Filter>, Action)>,
}

impl WConf {
    /// The rules of the settings in the order given, each setting one or more rules separated by
    /// commas (`WConf.fromSettings`, the settings reversed so that the rightmost wins).
    pub fn parse(settings: &[String]) -> Result<WConf, Vec<String>> {
        let rules: Vec<&str> = settings.iter().flat_map(|s| s.split(',')).filter(|r| !r.is_empty()).collect();
        let mut errors = Vec::new();
        let mut confs = Vec::new();
        for rule in rules.into_iter().rev() {
            let parts: Vec<&str> = rule.split(':').collect();
            if parts.len() != 2 {
                errors.push("exactly one `:` expected (<filter>&...&<filter>:<action>)".to_string());
                continue;
            }
            let action = match parts[1] {
                "error" | "e" => Ok(Action::Error),
                "warning" | "w" => Ok(Action::Warning),
                "verbose" | "v" => Ok(Action::Verbose),
                "info" | "i" => Ok(Action::Info),
                "silent" | "s" => Ok(Action::Silent),
                other => Err(vec![format!("unknown action: `{}`", other)]),
            };
            match Filter::parse_all(parts[0]).and_then(|f| action.map(|a| (f, a))) {
                Ok(conf) => confs.push(conf),
                Err(e) => errors.extend(e),
            }
        }
        if errors.is_empty() {
            Ok(WConf { confs })
        } else {
            Err(errors)
        }
    }

    pub fn action(&self, s: &Subject) -> Action {
        self.confs.iter().find(|(filters, _)| filters.iter().all(|f| f.matches(s))).map_or(Action::Default, |(_, a)| *a)
    }
}

/// The kinds of `--wunused`, scalac's `-Wunused` choices.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Unused {
    pub imports: bool,
    pub privates: bool,
    pub locals: bool,
    pub explicits: bool,
    pub implicits: bool,
    pub patvars: bool,
    pub nowarn: bool,
    /// Any choice was given (`WunusedHas.any`): the check runs.
    pub any: bool,
}

impl Unused {
    /// The choices of `--wunused <kinds>` (every one given, scalac's `-Wunused` adding to the
    /// choices of the one before), each kind what `WunusedHas` makes of the set; `Err` with the
    /// choices scalac does not know (`invalid choice(s) for -Wunused`), a kind taken back with a
    /// `-` among them.
    pub fn parse<'k>(kinds: impl IntoIterator<Item = &'k str>) -> Result<Unused, String> {
        const KNOWN: [&str; 12] = ["nowarn", "all", "imports", "privates", "locals", "explicits", "implicits", "params", "linted", "strict-no-implicit-warn", "unsafe-warn-patvars", "patvars"];
        let set: Vec<&str> = kinds.into_iter().flat_map(|k| k.split(',')).map(str::trim).filter(|k| !k.is_empty()).collect();
        let invalid: Vec<&str> = set.iter().copied().filter(|k| !KNOWN.contains(k)).collect();
        if !invalid.is_empty() {
            return Err(format!("invalid choice(s) for --wunused: {}", invalid.join(",")));
        }
        let has = |k: &str| set.contains(&k);
        let all_or = |k: &str| has("all") || has(k);
        let strict = has("strict-no-implicit-warn");
        Ok(Unused {
            imports: (all_or("imports") || all_or("linted")) && !strict,
            privates: all_or("privates") || all_or("linted"),
            locals: all_or("locals") || all_or("linted"),
            explicits: all_or("explicits") || all_or("params"),
            implicits: all_or("implicits") || all_or("params") || all_or("linted"),
            patvars: all_or("patvars") || has("unsafe-warn-patvars"),
            nowarn: all_or("nowarn"),
            any: !set.is_empty(),
        })
    }
}

/// What the build asks of its warnings: the flags of the conditional ones, `-Wconf`'s rules,
/// `--werror`, the kinds of `--wunused`.
#[derive(Clone, Default)]
pub struct Policy {
    pub deprecation: bool,
    pub feature: bool,
    pub tostring_interpolated: bool,
    /// The features `--language` enables (scalac's `-language`).
    pub language: Vec<String>,
    pub unused: Unused,
    /// The `--wconf` settings as given, and what they parse to (`Err` with the reasons).
    pub wconf_settings: Vec<String>,
    pub wconf: Option<Result<WConf, Vec<String>>>,
}

impl Policy {
    pub fn with_wconf(mut self, settings: Vec<String>) -> Policy {
        self.wconf = (!settings.is_empty()).then(|| WConf::parse(&settings));
        self.wconf_settings = settings;
        self
    }

    /// Whether a warning of `category` is summarized, its flag off.
    pub fn summarizes(&self, category: Category) -> bool {
        match category {
            Category::Deprecation => !self.deprecation,
            Category::Feature => !self.feature,
            _ => false,
        }
    }

    /// The message of `-Wconf` settings that do not parse (`WConf.parsed`).
    pub fn wconf_failure(&self) -> Option<String> {
        let Some(Err(reasons)) = &self.wconf else { return None };
        let multi = if self.wconf_settings.len() > 1 {
            "\nNote: for multiple filters, use `--wconf filter1:action1,filter2:action2`\n      or alternatively          `--wconf filter1:action1 --wconf filter2:action2`"
        } else {
            ""
        };
        Some(format!("Failed to parse `--wconf` configuration: {}\n{}{}", self.wconf_settings.join(","), reasons.join("\n"), multi))
    }

    pub fn action(&self, s: &Subject) -> Action {
        match &self.wconf {
            Some(Ok(w)) => w.action(s),
            _ => Action::Default,
        }
    }
}

/// The summary of the conditional warnings of a category its flag left out
/// (`Reporter.summarizeUnreportedWarnings`).
pub fn summary(category: Category, count: usize) -> Option<String> {
    let option = category.enabling_option()?;
    let were = if count == 1 { "was" } else { "were" };
    let warnings = if count == 1 { format!("1 {} warning", category.name()) } else { format!("{} {} warnings", count, category.name()) };
    Some(format!("there {} {}; re-run with {} for details", were, warnings, option))
}

/// The help verbose mode adds to a warning (`MessageRendering.appendFilterHelp`).
pub fn filter_help(id: u16, category: Category, origin: Option<&str>) -> String {
    let mut lines = Vec::new();
    if let Some(name) = ID_NAMES.get(id as usize) {
        lines.push(format!("  - id=E{}", id));
        lines.push(format!("  - name={}", name.trim_start_matches('-').strip_suffix("ID").unwrap_or(name)));
    }
    if category != Category::Plain {
        lines.push(format!("  - cat={}", category.name()));
    }
    if let (Category::Deprecation, Some(o)) = (category, origin) {
        lines.push(format!("  - origin={}", o));
    }
    if lines.is_empty() {
        return String::new();
    }
    format!("\nMatching filters for @nowarn or --wconf:\n{}", lines.join("\n"))
}

/// A `@nowarn` of a program file (`Suppression`): the range it covers, its annotation's span,
/// its filters.
#[derive(Clone)]
pub struct Suppression {
    pub file: FileId,
    pub annot: Span,
    pub start: u32,
    pub end: u32,
    pub filters: Vec<Filter>,
    pub verbose: bool,
}

impl Suppression {
    /// The suppression of the annotation at `annot` over `range` with the filter `conf`
    /// (`registerNowarn`): `Err` with the warning a filter that does not parse gives, the
    /// suppression then matching nothing.
    pub fn new(file: FileId, annot: Span, range: Span, conf: &str) -> (Suppression, Option<String>) {
        let mut verbose = false;
        let mut warning = None;
        let filters = match conf {
            "" => vec![Filter::Any],
            "none" => vec![Filter::None],
            "verbose" | "v" => {
                verbose = true;
                vec![Filter::Any]
            }
            _ => match Filter::parse_all(conf) {
                Ok(f) => f,
                Err(errors) => {
                    warning = Some(format!("Invalid message filter\n{}", errors.join("\n")));
                    vec![Filter::None]
                }
            },
        };
        (Suppression { file, annot, start: range.start, end: range.end, filters, verbose }, warning)
    }

    /// Whether the suppression is one of a filter that did not parse, which is never reported
    /// unused.
    pub fn invalid(&self) -> bool {
        matches!(self.filters.as_slice(), [Filter::None])
    }

    pub fn covers(&self, file: FileId, span: Span) -> bool {
        self.file == file && self.start <= span.start && span.end <= self.end
    }

    pub fn matches(&self, s: &Subject) -> bool {
        self.filters.iter().all(|f| f.matches(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subject(msg: &str, id: u16, category: Category) -> Subject<'_> {
        Subject { msg, id, category, origin: None, path: None }
    }

    #[test]
    fn the_rightmost_rule_wins() {
        let w = WConf::parse(&["id=E198:s".to_string(), "any:e".to_string()]).ok().unwrap();
        assert_eq!(w.action(&subject("unused import", 198, Category::Plain)), Action::Error);
        let w = WConf::parse(&["any:e,id=E198:s".to_string()]).ok().unwrap();
        assert_eq!(w.action(&subject("unused import", 198, Category::Plain)), Action::Silent);
        assert_eq!(w.action(&subject("other", 129, Category::Plain)), Action::Error);
    }

    #[test]
    fn filters_parse_as_scalac_parses_them() {
        assert!(matches!(Filter::parse("cat=unused"), Err(e) if e == "unknown category: unused"));
        assert!(matches!(Filter::parse("id=E0"), Err(e) if e == "E0 is marked as inactive."));
        assert!(matches!(Filter::parse("id=E999"), Err(e) if e == "Unknown error message number: E999"));
        assert!(matches!(Filter::parse("name=UnusedSymbol"), Ok(Filter::Id(198))));
        assert!(matches!(Filter::parse("id=198"), Ok(Filter::Id(198))));
        assert!(matches!(Filter::parse("msg=["), Err(e) if e.starts_with("invalid pattern `[`: Unclosed character class")));
        assert!(WConf::parse(&["any".to_string()]).is_err());
        assert_eq!(ID_NAMES[id::UNUSED_SYMBOL as usize], "UnusedSymbolID");
        assert_eq!(ID_NAMES[id::PATTERN_MATCH_EXHAUSTIVITY as usize], "PatternMatchExhaustivityID");
        assert_eq!(ID_NAMES[id::MATCH_IS_NOT_PARTIAL_FUNCTION as usize], "MatchIsNotPartialFunctionID");
        assert_eq!(ID_NAMES[id::FORMAT_INTERPOLATION_ERROR as usize], "FormatInterpolationErrorID");
        assert_eq!(ID_NAMES[id::PURE_EXPRESSION_IN_STATEMENT_POSITION as usize], "PureExpressionInStatementPositionID");
        assert_eq!(ID_NAMES[id::UNCHECKED_TYPE_PATTERN as usize], "UncheckedTypePatternID");
        assert_eq!(ID_NAMES[id::MATCH_CASE_UNREACHABLE as usize], "MatchCaseUnreachableID");
    }

    #[test]
    fn a_message_pattern_is_found_anywhere() {
        let f = Filter::parse("msg=nused imp").ok().unwrap();
        assert!(f.matches(&subject("unused import", 198, Category::Plain)));
        assert!(!f.matches(&subject("unreachable case", 30, Category::Plain)));
    }

    #[test]
    fn the_kinds_of_wunused() {
        let u = Unused::parse(["linted"]).unwrap();
        assert!(u.imports && u.privates && u.locals && u.implicits && !u.explicits && !u.patvars);
        let u = Unused::parse(["privates", "imports,locals"]).unwrap();
        assert!(u.imports && u.privates && u.locals && !u.implicits);
        assert_eq!(Unused::parse(["all,-imports"]).err().as_deref(), Some("invalid choice(s) for --wunused: -imports"));
        assert!(Unused::parse(["bogus"]).is_err());
        assert!(!Unused::parse([""]).unwrap().any);
    }
}
