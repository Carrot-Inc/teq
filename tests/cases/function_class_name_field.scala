// A class that is a function and has a field called `name`, which a JavaScript function owns
// read-only (munit's `SuiteTransform`); a `var` of that name assigned again, a subclass's.
final class Transform(val name: String, fn: Int => Int) extends (Int => Int):
  def apply(x: Int): Int = fn(x)

class Named(var name: String) extends (String => String):
  def apply(s: String): String = name + s

class Renamed extends Named("sub")

@main def run =
  val t = new Transform("double", _ * 2)
  println(s"${t.name} ${t(21)}")
  val n = new Named("a")
  n.name = "b"
  println(n("c"))
  val r = new Renamed
  println(r.name + " " + r("!"))
  val f: Int => Int = t
  println(f(5))
