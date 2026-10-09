// jars: scala-library abi-callbacks-lib
// std: scala-library
// Jar parents: a case class under a jar class with a final canEqual keeps it; an abstract class of
// the program that is the first to mix in a jar's stacked traits implements their super
// accessors for its subclasses; a jar abstract class that stacks them.
import abi.{Doubling, Incrementing, FinalEquals, Stacked}
case class FE(x: Int) extends FinalEquals
abstract class A extends Doubling
class B extends A
abstract class A2 extends Doubling with Incrementing
class B2 extends A2
class S2 extends Stacked
@main def run(): Unit =
  println(FE(1).canEqual(FE(2)))
  println(FE(1) == FE(1))
  println(B().m)
  println(B2().m)
  println(S2().m)
