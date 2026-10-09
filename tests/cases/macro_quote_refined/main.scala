// A quote pattern whose typed hole binds type variables through a refinement, as a mirror's
// element labels and types: `case '{ $m: Mirror.Product { type MirroredElemLabels = labels } }`.
package app
import mlib.Labels

final case class P(x: Int, name: String)
final case class Q(flag: Boolean)

@main def run(): Unit =
  println(Labels.of[P])
  println(Labels.of[Q])
