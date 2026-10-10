// A refinement's match-type alias names itself in its cases and stays named, as a class's does,
// and a sibling names it before its definition: no cycle (the forward-reference warning, which
// `@nowarn` silences here: tests/errors/refinement_forward_reference.scala has it).
import scala.annotation.nowarn

@nowarn type O1[A] = {
  type OutInner[Ts] <: Tuple = Ts match {
    case EmptyTuple => EmptyTuple
    case h *: t => h *: OutInner[t]
  }
  type Out = OutInner[A]
}

@main def run(): Unit =
  val t: O1[(Int, String)]#Out = (1, "two")
  println(t)
