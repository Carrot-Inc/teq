// A parent written with its type arguments and an annotation (`Inv[T] @uncheckedVariance`) is no parent named
// bare: its arguments are the class's own type parameters, read where the class's parents are resolved and
// nowhere else, for a class and for a trait alike.
import annotation.unchecked.uncheckedVariance
class Inv[U](val u: Int = 3)
class ContraC[-T] extends Inv[T] @uncheckedVariance
trait ContraT[-T] extends Inv[T] @uncheckedVariance
class Leaf extends ContraT[Any]
@main def run(): Unit =
  println(new ContraC[Int].u + new Leaf().u)
