/*
rules = DisableSyntax
DisableSyntax.noUniversalEquality = true
*/
// A type's annotation is a child of its `Type.Annotate`, its constructor and arguments with it (scalameta's; the
// review of `340567fdd`): the `==` of `@ann(1 == 2)` is reported.
package test.disableSyntax

class ann(x: Boolean) extends scala.annotation.StaticAnnotation
object AnnotatedTypeEquality {
  val x: String @ann(1 == 2) = "x" // assert: DisableSyntax.==
}
