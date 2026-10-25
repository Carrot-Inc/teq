// teq's build of observe.scala's per-compiler part (Scalac.scala is scalac's): teq's reflection
// (std/quoted/reflect.scala) declares `Flags` without `Invisible` and an import selector with no kind. Its trees have
// no import or export (src/interp/quoted.rs, `tree_kind`), so `selector` is never reached; were it reached, the
// observation would read `Unserialized`, which check.sh fails on.
package observe

import scala.quoted.*
import Observe.list

object Compiler:
  /** No mark: teq's reflection gives no bounds class (README.md, "bound kinds"). */
  def boundsMark(b: Any): String = ""

  /** A definition's flags, as Scalac.scala's but `Invisible`, which teq's reflection does not declare. */
  def flagNames(using Quotes)(s: quotes.reflect.Symbol): String =
    import quotes.reflect.*
    val all = List(
      "AbsOverride" -> Flags.AbsOverride, "Abstract" -> Flags.Abstract, "Artifact" -> Flags.Artifact,
      "Case" -> Flags.Case, "CaseAccessor" -> Flags.CaseAccessor, "Contravariant" -> Flags.Contravariant,
      "Covariant" -> Flags.Covariant, "Deferred" -> Flags.Deferred, "Enum" -> Flags.Enum, "Erased" -> Flags.Erased,
      "Exported" -> Flags.Exported, "ExtensionMethod" -> Flags.ExtensionMethod, "FieldAccessor" -> Flags.FieldAccessor,
      "Final" -> Flags.Final, "Given" -> Flags.Given, "HasDefault" -> Flags.HasDefault, "Implicit" -> Flags.Implicit,
      "Infix" -> Flags.Infix, "Inline" -> Flags.Inline,
      "JavaAnnotation" -> Flags.JavaAnnotation, "JavaDefined" -> Flags.JavaDefined, "JavaStatic" -> Flags.JavaStatic,
      "Lazy" -> Flags.Lazy, "Local" -> Flags.Local, "Macro" -> Flags.Macro, "Method" -> Flags.Method,
      "Module" -> Flags.Module, "Mutable" -> Flags.Mutable, "NoInits" -> Flags.NoInits, "Opaque" -> Flags.Opaque,
      "Open" -> Flags.Open, "Override" -> Flags.Override, "Package" -> Flags.Package, "Param" -> Flags.Param,
      "ParamAccessor" -> Flags.ParamAccessor, "Private" -> Flags.Private, "PrivateLocal" -> Flags.PrivateLocal,
      "Protected" -> Flags.Protected, "Scala2x" -> Flags.Scala2x, "Sealed" -> Flags.Sealed, "StableRealizable" -> Flags.StableRealizable,
      "Static" -> Flags.Static, "Synthetic" -> Flags.Synthetic, "Trait" -> Flags.Trait,
      "Transparent" -> Flags.Transparent)
    s"Flags(${list(all.collect { case (n, f) if s.flags.is(f) => n })})"

  def selectorOf(using Quotes)(s: quotes.reflect.Selector, tree: quotes.reflect.Tree => String): String = "Unserialized"
