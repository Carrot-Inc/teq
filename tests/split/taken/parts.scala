// What a module that is not per file takes from a per-file one under --module-per-file parts
// --hot, one of each kind: a class and a trait that a class of the other package extends, an
// object with a val and a lazy val, enum values (a case with parameters among them), a given,
// a top-level val with an initialiser, a var, a lazy val and a def.
package parts

trait Shape:
  def area: Int
  def describe: String = s"shape of $area"

class Base(val name: String):
  def greet: String = "base " + name

enum Kind:
  case Small, Large
  case Sized(n: Int)

object Registry:
  val items: List[String] = List("a", "b")
  lazy val total: Int =
    println("total computed")
    items.length

given defaultKind: Kind = Kind.Large

val top: String = "top " + Registry.items.mkString("+")
var counter: Int = 1
lazy val lazyTop: Int =
  println("lazyTop computed")
  40 + 2
def twice(n: Int): Int = n * 2
