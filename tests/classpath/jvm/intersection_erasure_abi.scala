// jars: scala-library erasure-lib
// std: scala-library
// The erasure of an intersection as a caller compiled by scalac links against it, read from the
// methods' descriptors (tests/support/erasure_lib.scala's reflection): a value class comes before
// any other part (scalac's `ErasedValueType`), two value classes are ordered by their classes, and
// two traits that nothing else orders by their full names, an enclosing object's with its `$`
// (`A$.Z` before `A+`).
package erasureabi

trait Tag extends Any
final class V(val r: Runnable) extends AnyVal with Tag
final class S(val s: String) extends AnyVal
final class W(val i: Int) extends AnyVal
object A:
  trait Z
trait `A+`
class C extends A.Z with `A+`

object Api:
  def take(v: V & Tag): Unit = v.r.run()
  def tagFirst(v: Tag & V): Unit = v.r.run()
  def twoValues(x: S & W): Unit = ()
  def valueArray(x: Array[Int] & S): Unit = ()
  def tie(x: A.Z & `A+`): String = "ok"

@main def main(): Unit =
  for name <- List("take", "tagFirst", "twoValues", "valueArray", "tie") do
    println(erasure.Descriptors.of("erasureabi.Api$", name))
  Api.take(new V(() => println("called")))
  println(Api.tie(new C))
