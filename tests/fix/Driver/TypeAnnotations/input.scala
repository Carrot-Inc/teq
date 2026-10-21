/*
rules = ["file:../../support/Annotations.scala"]
*/
// A type's annotation is a `Mod.Annot` of its `Type.Annotate` (the review of `340567fdd`), as a definition's is.
package annotations

class ann(x: Int) extends scala.annotation.StaticAnnotation
object TypeAnnotations {
  val x: String @ann(1) = "x" // assert: Annotations.annot
  @ann(2) val y = 1 // assert: Annotations.annot
}
