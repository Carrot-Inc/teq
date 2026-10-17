// The boundaries of a lambda's open result, refused as scalac refuses them: an inferred definition's
// type is complete (`Namer.inferredResultType`'s `fullyDefinedType`), whether stored in a `val`, captured
// by a nested closure or widened from a singleton; a lambda with no function type expected (the inner
// closure of `nested`) is wholly inferred; an overloaded call pretypes the lambda with its result
// complete (`Applications.resolveOverloaded`); an implicit search fixes the variable it searches over.
package lambdaopenresultrefusals

final case class Item[T](title: String, value: T, disabled: Boolean = false)
final case class Category(code: String, label: String)
object Category:
  val values: List[Category] = List(Category("flower", "Flower"), Category("edible", "Edible"))
class Box[T](val value: T)
def keep[A](f: String => (() => Item[A]), other: Item[A]): Item[A] = f("x")()
def choose(f: String => Item[Option[String]], n: Int): String = "option"
def choose(f: String => Item[Some[String]], n: String): String = "some"
trait Tag[A]:
  def name: String
given Tag[Option[String]] with
  def name = "option"
def inspect[A](xs: List[Item[A]])(using tag: Tag[A]): String = tag.name

def stored() =
  val xs = Category.values.map(c => Item(c.label, Some(c.code)))
  val wider: List[Item[Option[String]]] = xs
def captured() =
  keep(s => { val b = Item(s, Some(s)); () => b }, Item[Option[String]]("none", None))
def nested() =
  val xs: List[() => Item[Option[String]]] =
    (() => Item[Option[String]]("none", None)) :: List("x").map(s => () => Item(s, Some(s)))
def widened() =
  val f = (x: Int) => Box(1).value
  val v: 1 = f(0)
def overloaded() = choose(s => Item(s, Some(s)), 1)
def searched() = inspect(List("x").map(s => Item(s, Some(s))))
// expect: 24:43: error: type mismatch: found List[Item[Some[String]]], required List[Item[Option[String]]]
// expect: 26:52: error: type mismatch: found Item[Option[String]], required Item[Some[String]]
// expect: 29:12: error: type mismatch: found Item[Option[String]], required Item[Some[String]]
// expect: 32:14: error: type mismatch: found Int, required 1
// expect: 33:20: error: None of the overloaded alternatives of method choose
// expect: 34:63: error: no given instance of type Tag[Some[String]] was found for parameter tag
