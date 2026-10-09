use crate::intern::Name;
use crate::source::Span;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Tok {
    Eof,
    Newline,
    Indent,
    Outdent,

    Ident,
    OpIdent,
    IntLit,
    LongLit,
    DoubleLit,
    FloatLit,
    CharLit,
    StringLit,
    InterpStart,
    StrPart,
    InterpEnd,
    /// `'{` or `'[`: the quote that opens a quoted block or type.
    Quote,
    /// `'x`: a quoted identifier, whose name is on the token.
    QuoteId,
    /// `${`: the splice that opens a spliced block.
    Splice,

    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Semi,
    Dot,
    Colon,
    ColonEol,
    Eq,
    Arrow,
    CtxArrow,
    LArrow,
    Subtype,
    Supertype,
    At,
    Underscore,
    TypeLambdaArrow,

    KwAbstract,
    KwCase,
    KwClass,
    KwDef,
    KwDo,
    KwElse,
    KwEnum,
    KwExtends,
    KwFalse,
    KwFinal,
    KwFor,
    KwGiven,
    KwIf,
    KwImport,
    KwLazy,
    KwMatch,
    KwNew,
    KwNull,
    KwObject,
    KwOverride,
    KwPackage,
    KwPrivate,
    KwProtected,
    KwReturn,
    KwSealed,
    KwSuper,
    KwThen,
    KwThis,
    KwThrow,
    KwTrait,
    KwTrue,
    KwTry,
    KwCatch,
    KwFinally,
    KwType,
    KwVal,
    KwVar,
    KwWhile,
    KwWith,
    KwYield,
    KwExport,
    KwImplicit,
}

impl Tok {
    pub fn can_start_indent(self) -> bool {
        use Tok::*;
        matches!(
            self,
            Eq | Arrow | CtxArrow | LArrow | ColonEol | KwDo | KwElse | KwFor | KwIf | KwMatch
                | KwThen | KwWhile | KwYield | KwWith | KwReturn | KwTry | KwCatch | KwFinally
                | KwThrow
        )
    }

    /// dotc's `stopScanTokens`: what ends the search for the `then` or `do` of a condition
    /// that starts with a parenthesised expression.
    pub fn stops_cond_scan(self) -> bool {
        use Tok::*;
        matches!(
            self,
            KwDef | KwVal | KwVar | KwClass | KwObject | KwTrait | KwType | KwEnum | KwGiven
                | KwImport | KwExport | KwPackage | KwAbstract | KwFinal | KwSealed | KwLazy
                | KwOverride | KwPrivate | KwProtected | KwImplicit | KwIf | KwElse
                | KwWhile | KwDo | KwFor | KwYield | KwNew | KwTry | KwCatch | KwFinally | KwThrow
                | KwReturn | KwMatch | Semi | Eof
        )
    }

    pub fn can_end_statement(self) -> bool {
        use Tok::*;
        matches!(
            self,
            Ident | OpIdent | IntLit | LongLit | DoubleLit | FloatLit | CharLit | StringLit | InterpEnd
                | QuoteId | RParen | RBracket | RBrace | Underscore | KwThis | KwTrue | KwFalse | KwNull
                | KwReturn | KwType | KwGiven | Outdent
        )
    }

    pub fn can_start_statement(self) -> bool {
        use Tok::*;
        !matches!(
            self,
            KwElse | KwExtends | KwMatch | KwWith | KwYield | KwThen | KwDo | KwCatch
                | KwFinally | Comma | Dot | Semi | Colon | ColonEol | Eq | Arrow | CtxArrow
                | LArrow | Subtype | Supertype | RParen | RBracket | RBrace | Eof
                | TypeLambdaArrow
        )
    }

    pub fn describe(self) -> &'static str {
        use Tok::*;
        match self {
            Eof => "end of file",
            Newline => "new line",
            Indent => "indented block",
            Outdent => "end of indented block",
            Ident | OpIdent => "identifier",
            IntLit | LongLit | DoubleLit | FloatLit | CharLit | StringLit | InterpStart => "literal",
            StrPart | InterpEnd => "string part",
            Quote | QuoteId => "quote",
            Splice => "splice",
            LParen => "'('",
            RParen => "')'",
            LBracket => "'['",
            RBracket => "']'",
            LBrace => "'{'",
            RBrace => "'}'",
            Comma => "','",
            Semi => "';'",
            Dot => "'.'",
            Colon | ColonEol => "':'",
            Eq => "'='",
            Arrow => "'=>'",
            CtxArrow => "'?=>'",
            LArrow => "'<-'",
            Subtype => "'<:'",
            Supertype => "'>:'",
            At => "'@'",
            Underscore => "'_'",
            TypeLambdaArrow => "'=>>'",
            KwThen => "'then'",
            KwDo => "'do'",
            KwElse => "'else'",
            KwYield => "'yield'",
            KwCase => "'case'",
            KwDef => "'def'",
            KwVal => "'val'",
            KwMatch => "'match'",
            KwWith => "'with'",
            KwExtends => "'extends'",
            KwNew => "'new'",
            KwIf => "'if'",
            KwFor => "'for'",
            KwWhile => "'while'",
            KwAbstract => "'abstract'",
            KwClass => "'class'",
            KwEnum => "'enum'",
            KwFalse => "'false'",
            KwFinal => "'final'",
            KwGiven => "'given'",
            KwImport => "'import'",
            KwLazy => "'lazy'",
            KwNull => "'null'",
            KwObject => "'object'",
            KwOverride => "'override'",
            KwPackage => "'package'",
            KwPrivate => "'private'",
            KwProtected => "'protected'",
            KwReturn => "'return'",
            KwSealed => "'sealed'",
            KwSuper => "'super'",
            KwThis => "'this'",
            KwThrow => "'throw'",
            KwTrait => "'trait'",
            KwTrue => "'true'",
            KwTry => "'try'",
            KwCatch => "'catch'",
            KwFinally => "'finally'",
            KwType => "'type'",
            KwVar => "'var'",
            KwExport => "'export'",
            KwImplicit => "'implicit'",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Token {
    pub kind: Tok,
    pub span: Span,
    /// Interned text for identifiers and interpolator prefixes, EMPTY otherwise.
    pub name: Name,
}

pub fn keyword(s: &str) -> Option<Tok> {
    use Tok::*;
    Some(match s {
        "abstract" => KwAbstract,
        "case" => KwCase,
        "class" => KwClass,
        "def" => KwDef,
        "do" => KwDo,
        "else" => KwElse,
        "enum" => KwEnum,
        "extends" => KwExtends,
        "false" => KwFalse,
        "final" => KwFinal,
        "for" => KwFor,
        "given" => KwGiven,
        "if" => KwIf,
        "import" => KwImport,
        "lazy" => KwLazy,
        "match" => KwMatch,
        "new" => KwNew,
        "null" => KwNull,
        "object" => KwObject,
        "override" => KwOverride,
        "package" => KwPackage,
        "private" => KwPrivate,
        "protected" => KwProtected,
        "return" => KwReturn,
        "sealed" => KwSealed,
        "super" => KwSuper,
        "then" => KwThen,
        "this" => KwThis,
        "throw" => KwThrow,
        "trait" => KwTrait,
        "true" => KwTrue,
        "try" => KwTry,
        "catch" => KwCatch,
        "finally" => KwFinally,
        "type" => KwType,
        "val" => KwVal,
        "var" => KwVar,
        "while" => KwWhile,
        "with" => KwWith,
        "yield" => KwYield,
        "export" => KwExport,
        "implicit" => KwImplicit,
        _ => return None,
    })
}
