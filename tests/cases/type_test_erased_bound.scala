// A type pattern over an abstract type without a tag tests the erasure of its upper bound, as
// scalac erases the tested type before the test (TypeTestsCasts.scala 359-360; TypeErasure.scala
// 767-819): a literal bound its class, a union bound the erased lub (`A | B` below `Base` takes
// every `Base`), an intersection bound the erased glb (`A & B` takes every `A`), where a
// composite written as the pattern keeps its own test. `null` passes none.
trait Base
trait A extends Base
trait B extends Base
class OnlyA extends A
class OnlyB extends B
class Both extends A with B
class Other extends Base
object Main:
  def lit[T <: 1](x: Any) = x match { case _: T @unchecked => true; case _ => false }
  def union[T <: A | B](x: Any) = x match { case _: T @unchecked => true; case _ => false }
  def inter[T <: A & B](x: Any) = x match { case _: T @unchecked => true; case _ => false }
  def written(x: Any) = x match { case _: (A | B) => true; case _ => false }
  def writtenBoth(x: Any) = x match { case _: (A & B) => true; case _ => false }
  def nested[T <: 1, U <: T](x: Any) = x match { case _: U @unchecked => true; case _ => false }
  def main(args: Array[String]): Unit =
    println(lit[1](1)); println(lit[1](2)); println(lit[1](2.5)); println(lit[1]("s")); println(lit[1](null))
    println(nested[1, 1](7)); println(nested[1, 1]("s"))
    for x <- List[Any](OnlyA(), OnlyB(), Both(), Other(), "s", null) do
      println(s"${union[A](x)} ${inter[Both](x)} ${written(x)} ${writtenBoth(x)}")
