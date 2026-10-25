// What observe.scala reads through the parts of 3.8.4's public reflection teq's declares otherwise
// (std/quoted/reflect.scala): an import's selector kinds (Quotes.scala 2560-2678; teq declares one `Selector` class
// with no kind) and `Flags.Invisible` (teq has the rest). scalac's build of the observer compiles this file; teq's
// compiles Teq.scala, whose arms its reflection never reaches (README.md, "The observation").
package observe

import scala.quoted.*
import Observe.{hit, list, name, option}

object Compiler:
  /** A bounds object's dotty class (`RealTypeBounds`, `TypeAlias`, `MatchAlias`), which the public API does not tell
   *  apart, by the class itself, a type test on scalac's own (README.md, "bound kinds"); check.sh strips the mark
   *  before comparing a scalac observation with teq's. */
  def boundsMark(b: Any): String = b match
    case _: dotty.tools.dotc.core.Types.MatchAlias => "{MatchAlias}"
    case _: dotty.tools.dotc.core.Types.TypeAlias => "{TypeAlias}"
    case _: dotty.tools.dotc.core.Types.RealTypeBounds => "{RealTypeBounds}"
    case _ => ""

  /** A definition's flags, each of 3.8.4's `Flags` it carries by its name, in this fixed order. */
  def flagNames(using Quotes)(s: quotes.reflect.Symbol): String =
    import quotes.reflect.*
    val all = List(
      "AbsOverride" -> Flags.AbsOverride, "Abstract" -> Flags.Abstract, "Artifact" -> Flags.Artifact,
      "Case" -> Flags.Case, "CaseAccessor" -> Flags.CaseAccessor, "Contravariant" -> Flags.Contravariant,
      "Covariant" -> Flags.Covariant, "Deferred" -> Flags.Deferred, "Enum" -> Flags.Enum, "Erased" -> Flags.Erased,
      "Exported" -> Flags.Exported, "ExtensionMethod" -> Flags.ExtensionMethod, "FieldAccessor" -> Flags.FieldAccessor,
      "Final" -> Flags.Final, "Given" -> Flags.Given, "HasDefault" -> Flags.HasDefault, "Implicit" -> Flags.Implicit,
      "Infix" -> Flags.Infix, "Inline" -> Flags.Inline, "Invisible" -> Flags.Invisible,
      "JavaAnnotation" -> Flags.JavaAnnotation, "JavaDefined" -> Flags.JavaDefined, "JavaStatic" -> Flags.JavaStatic,
      "Lazy" -> Flags.Lazy, "Local" -> Flags.Local, "Macro" -> Flags.Macro, "Method" -> Flags.Method,
      "Module" -> Flags.Module, "Mutable" -> Flags.Mutable, "NoInits" -> Flags.NoInits, "Opaque" -> Flags.Opaque,
      "Open" -> Flags.Open, "Override" -> Flags.Override, "Package" -> Flags.Package, "Param" -> Flags.Param,
      "ParamAccessor" -> Flags.ParamAccessor, "Private" -> Flags.Private, "PrivateLocal" -> Flags.PrivateLocal,
      "Protected" -> Flags.Protected, "Scala2x" -> Flags.Scala2x, "Sealed" -> Flags.Sealed, "StableRealizable" -> Flags.StableRealizable,
      "Static" -> Flags.Static, "Synthetic" -> Flags.Synthetic, "Trait" -> Flags.Trait,
      "Transparent" -> Flags.Transparent)
    s"Flags(${list(all.collect { case (n, f) if s.flags.is(f) => n })})"

  /** An import's or an export's selector by its kind. */
  def selectorOf(using Quotes)(s: quotes.reflect.Selector, tree: quotes.reflect.Tree => String): String =
    import quotes.reflect.*
    s match
      // An omitting selector is a renaming one to `_` too (QuotesImpl 1770-1774, 1793-1797): tested first.
      case OmitSelector(n) => hit("OmitSelector"); s"Omit(${name(n)})"
      case SimpleSelector(n) => hit("SimpleSelector"); s"Simple(${name(n)})"
      case RenameSelector(from, to) => hit("RenameSelector"); s"Rename(${name(from)}, ${name(to)})"
      case GivenSelector(bound) => hit("GivenSelector"); s"GivenSelector(${option(bound.map(tree))})"
