// The shape of scala3's tests/pos/i6146.scala (`_` written as `?`): an F-bound over a wildcard (`S <: BS[?, S]`)
// checked while `S`'s own bound is completing, which dotty admits through the GADT bound it installs for `S`
// (`narrowBound`, TypeOps.scala 637-683). scalac 3.8.4 accepts it.
trait BS[T, S <: BS[T, S]]
trait IS extends BS[Int, IS]

sealed trait BSElem[T, S <: BS[?, S]]

object BSElem {
  implicit val intStreamShape: BSElem[Int, IS] = new BSElem[Int, IS] {}
}
class Ops[A] {
  def asJavaSeqStream[S <: BS[?, S]](implicit s: BSElem[A, S]): String = if s eq BSElem.intStreamShape then "IS shape" else "other"
}
@main def main(): Unit = println(Ops[Int]().asJavaSeqStream[IS])
