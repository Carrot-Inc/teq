// The ordinal function of a sum's mirror takes the value in a parameter named `a$0`, which it
// tests against the case class named as it is.
import scala.deriving.Mirror

sealed trait T
case class `a$0`() extends T
case class B() extends T

@main def main(): Unit =
  val m = summon[Mirror.SumOf[T]]
  println(m.ordinal(B()))
  println(m.ordinal(`a$0`()))
