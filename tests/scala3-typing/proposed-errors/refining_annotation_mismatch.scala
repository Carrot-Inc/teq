// A refining annotation (`RefiningAnnotation`) is part of its type: `TypeComparer.thirdTry` compares an `AnnotatedType`
// on the right by its annotation too (903-905 at 3.8.4), and `fourthTry` one on the left (1117), so scalac 3.8.4
// refuses a value without the annotation or with another one, E007 at both. teq drops an annotation from the canonical
// type and accepts both under the scala library (its lean std has no `RefiningAnnotation`). `AnnotatedType` as the
// type is type-representation's (the pass over fix-semantic's 32fc738c3); this case goes live at the rebase onto its
// landing. scalac's lines.
// teq: --std=scala-library
// expect: 15:22: error: type mismatch: found 1, required Int @Ref
// expect: 16:25: error: type mismatch: found Int @Other, required Int @Ref
package review
class Ref extends scala.annotation.RefiningAnnotation
class Other extends scala.annotation.RefiningAnnotation
object Refining:
  val good: Int @Ref = (??? : Int @Ref)
  val bad: Int @Ref = 1
  val wrong: Int @Ref = (??? : Int @Other)
