/*
rules = DisableSyntax
DisableSyntax.noCovariantTypes = true
*/
// A higher-kinded parameter's own parameter reads its annotations first (dotty's `typeParam`, `annotsAsMods`; the
// review of `86884ea36`), so `F[@ann +A]` parses, its `@ann` a `Mod.Annot` and its `+` reported.
package test.disableSyntax

class ann extends scala.annotation.StaticAnnotation
trait Higher[F[@ann +A]] // assert: DisableSyntax.covariant
