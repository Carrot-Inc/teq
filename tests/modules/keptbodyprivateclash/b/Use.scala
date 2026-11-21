import lib.*
final case class P(i: Int)
given Eqv[P] with
  def eqv(x: P, y: P) = x == y
@main def main() = println(Inst.eqL[P].eqv(List(P(1)), List(P(1))))
