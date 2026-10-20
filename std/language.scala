// scala-library 3.8.4's `scala.languageFeature` and `scala.language` (library/src/scala/languageFeature.scala,
// library/src/scala/language.scala): the objects a language import names, which the typer checks an import
// that looks like one refers to (`Checking.checkLegalImportPath`); the features themselves are read off the
// import's syntax (`ImportInfo.isLanguageImport`, `Ast::language_scopes`).
package scala

import scala.annotation.meta
import scala.annotation.compileTimeOnly

object languageFeature {
  @meta.languageFeature("extension of type scala.Dynamic", enableRequired = true)
  sealed trait dynamics
  object dynamics extends dynamics

  @meta.languageFeature("postfix operator #", enableRequired = true)
  sealed trait postfixOps
  object postfixOps extends postfixOps

  @meta.languageFeature("reflective access of structural type member #", enableRequired = false)
  sealed trait reflectiveCalls
  object reflectiveCalls extends reflectiveCalls

  @meta.languageFeature("implicit conversion #", enableRequired = false)
  sealed trait implicitConversions
  object implicitConversions extends implicitConversions

  @deprecated("scala.language.higherKinds no longer needs to be imported explicitly", "2.13.1")
  @meta.languageFeature("higher-kinded type", enableRequired = false)
  sealed trait higherKinds
  @deprecated("scala.language.higherKinds no longer needs to be imported explicitly", "2.13.1")
  object higherKinds extends higherKinds

  @meta.languageFeature("#, which cannot be expressed by wildcards,", enableRequired = false)
  sealed trait existentials
  object existentials extends existentials

  object experimental {
    @meta.languageFeature("macro definition", enableRequired = true)
    sealed trait macros
    object macros extends macros
  }
}

object language {
  import languageFeature.*

  implicit lazy val dynamics: dynamics = languageFeature.dynamics
  implicit lazy val postfixOps: postfixOps = languageFeature.postfixOps
  implicit lazy val reflectiveCalls: reflectiveCalls = languageFeature.reflectiveCalls
  implicit lazy val implicitConversions: implicitConversions = languageFeature.implicitConversions
  @deprecated("higherKinds no longer needs to be imported explicitly", "2.13.1")
  implicit lazy val higherKinds: higherKinds = languageFeature.higherKinds
  implicit lazy val existentials: existentials = languageFeature.existentials

  object experimental {
    import languageFeature.experimental.*

    implicit lazy val macros: macros = languageFeature.experimental.macros

    @compileTimeOnly("`namedTypeArguments` can only be used at compile time in import statements")
    object namedTypeArguments
    @compileTimeOnly("`genericNumberLiterals` can only be used at compile time in import statements")
    object genericNumberLiterals
    @compileTimeOnly("`erasedDefinitions` can only be used at compile time in import statements")
    object erasedDefinitions
    @compileTimeOnly("`strictEqualityPatternMatching` can only be used at compile time in import statements")
    object strictEqualityPatternMatching
    @compileTimeOnly("`fewerBraces` can only be used at compile time in import statements")
    @deprecated("`fewerBraces` is now standard, no language import is needed", since = "3.3")
    object fewerBraces
    @compileTimeOnly("`saferExceptions` can only be used at compile time in import statements")
    object saferExceptions
    @compileTimeOnly("`clauseInterleaving` can only be used at compile time in import statements")
    @deprecated("`clauseInterleaving` is now standard, no language import is needed", since = "3.6")
    object clauseInterleaving
    @compileTimeOnly("`pureFunctions` can only be used at compile time in import statements")
    object pureFunctions
    @compileTimeOnly("`captureChecking` can only be used at compile time in import statements")
    object captureChecking
    @compileTimeOnly("`separationChecking` can only be used at compile time in import statements")
    object separationChecking
    @compileTimeOnly("`into` can only be used at compile time in import statements")
    @deprecated("The into language import is no longer needed since the feature is now in preview", since = "3.8")
    object into
    @compileTimeOnly("`namedTuples` can only be used at compile time in import statements")
    @deprecated("The experimental.namedTuples language import is no longer needed since the feature is now standard", since = "3.7")
    object namedTuples
    @compileTimeOnly("`modularity` can only be used at compile time in import statements")
    object modularity
    @compileTimeOnly("`relaxedExtensionImports` can only be used at compile time in import statements")
    @deprecated("The experimental.relaxedExtensionImports language import is no longer needed since the feature is now standard", since = "3.4")
    object relaxedExtensionImports
    @compileTimeOnly("`betterMatchTypeExtractors` can only be used at compile time in import statements")
    @deprecated("The experimental.betterMatchTypeExtractors language import is no longer needed since the feature is now standard. It now has no effect, including when setting an older source version.", since = "3.6")
    object betterMatchTypeExtractors
    @compileTimeOnly("`quotedPatternsWithPolymorphicFunctions` can only be used at compile time in import statements")
    object quotedPatternsWithPolymorphicFunctions
    @compileTimeOnly("`betterFors` can only be used at compile time in import statements")
    @deprecated("The `experimental.betterFors` language import no longer has any effect, the feature is being stabilised and can be enabled using `-preview` flag", since = "3.7")
    object betterFors
    @compileTimeOnly("`packageObjectValues` can only be used at compile time in import statements")
    object packageObjectValues
    @compileTimeOnly("`multiSpreads` can only be used at compile time in import statements")
    object multiSpreads
    @compileTimeOnly("`subCases` can only be used at compile time in import statements")
    object subCases
    @compileTimeOnly("`relaxedLambdaSyntax` can only be used at compile time in import statements")
    object relaxedLambdaSyntax
    @compileTimeOnly("`safe` can only be used at compile time in import statements")
    object safe
  }

  @compileTimeOnly("`deprecated` can only be used at compile time in import statements")
  object deprecated {
    @compileTimeOnly("`symbolLiterals` can only be used at compile time in import statements")
    object symbolLiterals
  }

  @compileTimeOnly("`noAutoTupling` can only be used at compile time in import statements")
  object noAutoTupling
  @compileTimeOnly("`strictEquality` can only be used at compile time in import statements")
  object strictEquality
  @compileTimeOnly("`adhocExtensions` can only be used at compile time in import statements")
  object adhocExtensions
  @compileTimeOnly("`unsafeNulls` can only be used at compile time in import statements")
  object unsafeNulls
  @compileTimeOnly("`future` can only be used at compile time in import statements")
  object future
  @compileTimeOnly("`future-migration` can only be used at compile time in import statements")
  object `future-migration`
  @compileTimeOnly("`2.13` can only be used at compile time in import statements")
  private[scala] object `2.13`
  @compileTimeOnly("`3.0-migration` can only be used at compile time in import statements")
  object `3.0-migration`
  @compileTimeOnly("`3.0` can only be used at compile time in import statements")
  object `3.0`
  @compileTimeOnly("`3.1-migration` can only be used at compile time in import statements")
  @deprecated("`3.1-migration` is not valid, use `3.1` instead", since = "3.2")
  object `3.1-migration`
  @compileTimeOnly("`3.1` can only be used at compile time in import statements")
  object `3.1`
  @compileTimeOnly("`3.2-migration` can only be used at compile time in import statements")
  object `3.2-migration`
  @compileTimeOnly("`3.2` can only be used at compile time in import statements")
  object `3.2`
  @compileTimeOnly("`3.3-migration` can only be used at compile time in import statements")
  object `3.3-migration`
  @compileTimeOnly("`3.3` can only be used at compile time in import statements")
  object `3.3`
  @compileTimeOnly("`3.4-migration` can only be used at compile time in import statements")
  object `3.4-migration`
  @compileTimeOnly("`3.4` can only be used at compile time in import statements")
  object `3.4`
  @compileTimeOnly("`3.5-migration` can only be used at compile time in import statements")
  object `3.5-migration`
  @compileTimeOnly("`3.5` can only be used at compile time in import statements")
  object `3.5`
  @compileTimeOnly("`3.6-migration` can only be used at compile time in import statements")
  object `3.6-migration`
  @compileTimeOnly("`3.6` can only be used at compile time in import statements")
  object `3.6`
  @compileTimeOnly("`3.7-migration` can only be used at compile time in import statements")
  object `3.7-migration`
  @compileTimeOnly("`3.7` can only be used at compile time in import statements")
  object `3.7`
  @compileTimeOnly("`3.8-migration` can only be used at compile time in import statements")
  object `3.8-migration`
  @compileTimeOnly("`3.8` can only be used at compile time in import statements")
  object `3.8`
  @compileTimeOnly("`3.9-migration` can only be used at compile time in import statements")
  object `3.9-migration`
  @compileTimeOnly("`3.9` can only be used at compile time in import statements")
  object `3.9`
}
