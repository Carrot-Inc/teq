//! Tag values of `dotty.tools.tasty.TastyFormat`. A tree's tag tells its layout: 1-59 the tag
//! alone, 60-89 a Nat, 90-109 a tree, 110-127 a Nat and a tree, 128-255 a length and a payload.

#![allow(dead_code)]

pub mod name {
    pub const UTF8: u8 = 1;
    pub const QUALIFIED: u8 = 2;
    pub const EXPANDED: u8 = 3;
    pub const EXPANDPREFIX: u8 = 4;
    pub const UNIQUE: u8 = 10;
    pub const DEFAULTGETTER: u8 = 11;
    pub const SUPERACCESSOR: u8 = 20;
    pub const INLINEACCESSOR: u8 = 21;
    pub const BODYRETAINER: u8 = 22;
    pub const OBJECTCLASS: u8 = 23;
    pub const TARGETSIGNED: u8 = 62;
    pub const SIGNED: u8 = 63;
}

pub mod attr {
    pub const SCALA2_STANDARD_LIBRARY: u8 = 1;
    pub const JAVA: u8 = 5;
}

pub const UNITCONST: u8 = 2;
pub const FALSECONST: u8 = 3;
pub const TRUECONST: u8 = 4;
pub const NULLCONST: u8 = 5;
pub const PRIVATE: u8 = 6;
pub const PROTECTED: u8 = 8;
pub const ABSTRACT: u8 = 9;
pub const FINAL: u8 = 10;
pub const SEALED: u8 = 11;
pub const CASE: u8 = 12;
pub const IMPLICIT: u8 = 13;
pub const LAZY: u8 = 14;
pub const OVERRIDE: u8 = 15;
pub const INLINEPROXY: u8 = 16;
pub const INLINE: u8 = 17;
pub const STATIC: u8 = 18;
pub const OBJECT: u8 = 19;
pub const TRAIT: u8 = 20;
pub const ENUM: u8 = 21;
pub const LOCAL: u8 = 22;
pub const SYNTHETIC: u8 = 23;
pub const ARTIFACT: u8 = 24;
pub const MUTABLE: u8 = 25;
pub const FIELDACCESSOR: u8 = 26;
pub const CASEACCESSOR: u8 = 27;
pub const COVARIANT: u8 = 28;
pub const CONTRAVARIANT: u8 = 29;
pub const HASDEFAULT: u8 = 31;
pub const STABLE: u8 = 32;
pub const MACRO: u8 = 33;
pub const ERASED: u8 = 34;
pub const OPAQUE: u8 = 35;
pub const EXTENSION: u8 = 36;
pub const GIVEN: u8 = 37;
pub const PARAMSETTER: u8 = 38;
pub const EXPORTED: u8 = 39;
pub const OPEN: u8 = 40;
pub const PARAMALIAS: u8 = 41;
pub const TRANSPARENT: u8 = 42;
pub const INFIX: u8 = 43;
pub const INVISIBLE: u8 = 44;
pub const EMPTYCLAUSE: u8 = 45;
pub const SPLITCLAUSE: u8 = 46;
pub const TRACKED: u8 = 47;
pub const SUBMATCH: u8 = 48;
pub const INTO: u8 = 49;

pub const SHAREDTERM: u8 = 60;
pub const SHAREDTYPE: u8 = 61;
pub const TERMREFDIRECT: u8 = 62;
pub const TYPEREFDIRECT: u8 = 63;
pub const TERMREFPKG: u8 = 64;
pub const TYPEREFPKG: u8 = 65;
pub const RECTHIS: u8 = 66;
pub const BYTECONST: u8 = 67;
pub const SHORTCONST: u8 = 68;
pub const CHARCONST: u8 = 69;
pub const INTCONST: u8 = 70;
pub const LONGCONST: u8 = 71;
pub const FLOATCONST: u8 = 72;
pub const DOUBLECONST: u8 = 73;
pub const STRINGCONST: u8 = 74;
pub const IMPORTED: u8 = 75;
pub const RENAMED: u8 = 76;

pub const THIS: u8 = 90;
pub const QUALTHIS: u8 = 91;
pub const CLASSCONST: u8 = 92;
pub const BYNAMETYPE: u8 = 93;
pub const BYNAMETPT: u8 = 94;
pub const NEW: u8 = 95;
pub const THROW: u8 = 96;
pub const IMPLICITARG: u8 = 97;
pub const PRIVATEQUALIFIED: u8 = 98;
pub const PROTECTEDQUALIFIED: u8 = 99;
pub const RECTYPE: u8 = 100;
pub const SINGLETONTPT: u8 = 101;
pub const BOUNDED: u8 = 102;
pub const EXPLICITTPT: u8 = 103;
pub const ELIDED: u8 = 104;

pub const IDENT: u8 = 110;
pub const IDENTTPT: u8 = 111;
pub const SELECT: u8 = 112;
pub const SELECTTPT: u8 = 113;
pub const TERMREFSYMBOL: u8 = 114;
pub const TERMREF: u8 = 115;
pub const TYPEREFSYMBOL: u8 = 116;
pub const TYPEREF: u8 = 117;
pub const SELFDEF: u8 = 118;
pub const NAMEDARG: u8 = 119;

pub const PACKAGE: u8 = 128;
pub const VALDEF: u8 = 129;
pub const DEFDEF: u8 = 130;
pub const TYPEDEF: u8 = 131;
pub const IMPORT: u8 = 132;
pub const TYPEPARAM: u8 = 133;
pub const PARAM: u8 = 134;
pub const APPLY: u8 = 136;
pub const TYPEAPPLY: u8 = 137;
pub const TYPED: u8 = 138;
pub const ASSIGN: u8 = 139;
pub const BLOCK: u8 = 140;
pub const IF: u8 = 141;
pub const LAMBDA: u8 = 142;
pub const MATCH: u8 = 143;
pub const RETURN: u8 = 144;
pub const WHILE: u8 = 145;
pub const TRY: u8 = 146;
pub const INLINED: u8 = 147;
pub const SELECTOUTER: u8 = 148;
pub const REPEATED: u8 = 149;
pub const BIND: u8 = 150;
pub const ALTERNATIVE: u8 = 151;
pub const UNAPPLY: u8 = 152;
pub const ANNOTATEDTYPE: u8 = 153;
pub const ANNOTATEDTPT: u8 = 154;
pub const CASEDEF: u8 = 155;
pub const TEMPLATE: u8 = 156;
pub const SUPER: u8 = 157;
pub const SUPERTYPE: u8 = 158;
pub const REFINEDTYPE: u8 = 159;
pub const REFINEDTPT: u8 = 160;
pub const APPLIEDTYPE: u8 = 161;
pub const APPLIEDTPT: u8 = 162;
pub const TYPEBOUNDS: u8 = 163;
pub const TYPEBOUNDSTPT: u8 = 164;
pub const ANDTYPE: u8 = 165;
pub const ORTYPE: u8 = 167;
pub const POLYTYPE: u8 = 169;
pub const TYPELAMBDATYPE: u8 = 170;
pub const LAMBDATPT: u8 = 171;
pub const PARAMTYPE: u8 = 172;
pub const ANNOTATION: u8 = 173;
pub const TERMREFIN: u8 = 174;
pub const TYPEREFIN: u8 = 175;
pub const SELECTIN: u8 = 176;
pub const EXPORT: u8 = 177;
pub const QUOTE: u8 = 178;
pub const SPLICE: u8 = 179;
pub const METHODTYPE: u8 = 180;
pub const APPLYSIGPOLY: u8 = 181;
pub const QUOTEPATTERN: u8 = 182;
pub const SPLICEPATTERN: u8 = 183;
pub const MATCHTYPE: u8 = 190;
pub const MATCHTPT: u8 = 191;
pub const MATCHCASETYPE: u8 = 192;
pub const FLEXIBLETYPE: u8 = 193;
pub const HOLE: u8 = 255;

pub fn is_modifier(tag: u8) -> bool {
    matches!(
        tag,
        PRIVATE
            | PROTECTED
            | ABSTRACT
            | FINAL
            | SEALED
            | CASE
            | IMPLICIT
            | GIVEN
            | ERASED
            | LAZY
            | OVERRIDE
            | INLINE
            | INLINEPROXY
            | MACRO
            | OPAQUE
            | STATIC
            | OBJECT
            | TRAIT
            | TRANSPARENT
            | INFIX
            | ENUM
            | LOCAL
            | SYNTHETIC
            | ARTIFACT
            | MUTABLE
            | FIELDACCESSOR
            | CASEACCESSOR
            | COVARIANT
            | CONTRAVARIANT
            | HASDEFAULT
            | STABLE
            | EXTENSION
            | PARAMSETTER
            | PARAMALIAS
            | EXPORTED
            | OPEN
            | INVISIBLE
            | ANNOTATION
            | PRIVATEQUALIFIED
            | PROTECTEDQUALIFIED
            | TRACKED
            | INTO
    )
}
