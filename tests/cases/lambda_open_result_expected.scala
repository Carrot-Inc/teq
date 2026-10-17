// An invariant result variable of a lambda fixed by an expected type that arrives later: a method's
// declared result, an assignment, a later argument, a by-name argument (evaluated once) and a lambda
// nested in a lambda.
package lambdaopenresultexpected

final case class Item[T](title: String, value: T, disabled: Boolean = false)
final case class Category(code: String, label: String)
object Category:
  val values: List[Category] = List(Category("flower", "Flower"), Category("edible", "Edible"))

def items: List[Item[Option[String]]] = Category.values.map(c => Item(c.label, Some(c.code)))
def choose[A](f: String => Item[A], other: Item[A]): Item[A] = f("x")
def chooseByName[A](x: => Item[A], other: Item[A]): Item[A] = x
def keep[A](f: String => (() => Item[A]), other: Item[A]): Item[A] = f("x")()

@main def main(): Unit =
  println(items.map(_.value))
  var xs: List[Item[Option[String]]] = Nil
  xs = Category.values.map(c => Item(c.label, Some(c.code)))
  println(xs.map(_.value))
  println(choose(s => Item(s, Some(s)), Item[Option[String]]("none", None)).value)
  var n = 0
  val byName = chooseByName({ n += 1; Item("x", Some("x")) }, Item[Option[String]]("none", None))
  println((byName.value, n))
  println(keep(s => () => Item(s, Some(s)), Item[Option[String]]("none", None)).value)
