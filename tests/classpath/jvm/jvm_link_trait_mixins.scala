// jars: scala-library abi-callbacks-lib
// std: scala-library
// Stacked traits of a jar mixed into the program's classes: the class implements the super
// accessors (abi$Doubling$$super$m) of the jar traits it is the first to mix in, and forwards to
// the default that wins in its linearisation where two unrelated traits define it.
import abi.{Base, Doubling, Incrementing}
class P extends Doubling with Incrementing
class Q extends Incrementing with Doubling
class R extends Doubling
trait Tripling extends Base:
  override def m: Int = super.m * 3
class S extends Doubling with Tripling
@main def run(): Unit =
  println(P().m)
  println(Q().m)
  println(R().m)
  println(S().m)
