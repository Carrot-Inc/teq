// Adapted from scala3 tests/neg/i17132.scala (Apache-2.0, see tests/scala3/README.md):
// a case class whose parent applies a match alias to the class's own parameters,
// the alias's case naming the class. scalac 3.8.4 rejects it while emitting,
// the reduction of `(t1, t2) match` recurring to the recursion limit ("Maybe
// there is an illegal cyclic reference?"), its outcome the .expected: GenBCode's
// `GenericSignatures.jsig` (GenericSignatures.scala:336) erases the parent
// `Transformation[MultiTransformationResult[T1, T2]]` through `TypeErasure.erasure`
// (TypeErasure.scala:217, `apply` 779, `checkedSuperType` 902), whose `translucentSuperType`
// normalizes the match type; with `-scalajs` too. Pending until the JVM backend's generic
// signatures, queued as their own part (the harness pins neg/i17132 to it). dotty never
// bound-checks the case pattern's `MultiTransformation[t1, t2]` (PostTyper's `Mode.Pattern`).

sealed trait Transformation[T]

case object Count extends Transformation[Int]
case class MultiTransformation[T1 <: Transformation[?], T2 <: Transformation[?]](t1: T1, t2: T2) // error cyclic
  extends Transformation[MultiTransformationResult[T1, T2]]

type MultiTransformationResult[T1 <: Transformation[?], T2 <: Transformation[?]] <: Tuple = (T1, T2) match {
  case (Transformation[t], MultiTransformation[t1, t2]) => t *: MultiTransformationResult[t1, t2]
  case (Transformation[t1], Transformation[t2]) => (t1, t2)
}
