// expect: no given instance of type TC[Int]
// expect: not found: intTC
// expect: not found: plain
// expect: value label is not a member of Secret
// expect: no given instance of type Label[Secret]
package demo

trait TC[A]:
  def name: String
final class Impl[A](val name: String) extends TC[A]

trait Label[A]:
  extension (a: A) def label: String

object Wildcard:
  given intTC: TC[Int] = Impl("int")
  val visible: Int = 1

object GivenOnly:
  given stringTC: TC[String] = Impl("string")
  val plain: Int = 2

final case class Secret(code: Int)
object Secret:
  private given Label[Secret] with
    extension (s: Secret) def label: String = "secret"

// A wildcard import leaves the givens out, and a given import brings in nothing else.
import Wildcard.*
import GivenOnly.given

def labelOf[A](a: A)(using l: Label[A]): String = a.label

@main def run(): Unit =
  println(visible)
  println(summon[TC[String]].name + stringTC.name)
  println(summon[TC[Int]].name)
  println(intTC.name)
  println(plain)
  println(Secret(1).label)
  println(labelOf(Secret(1)))
