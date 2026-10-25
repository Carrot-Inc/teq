// The oracle's refining-annotation fixture (README.md), apart from api.scala because the lean std has no
// `scala.annotation.RefiningAnnotation`: dotty's tests/pos/annot-refining-avoid.scala's `Int @myAnnot`.
package repr.refining

import scala.annotation.RefiningAnnotation

class myAnnot extends RefiningAnnotation

object Refining:
  type Refined = Int @myAnnot
  def refined: Refined = ???
  def direct: Int @myAnnot = ???
