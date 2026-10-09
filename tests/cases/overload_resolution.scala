//> using scala 3.8.4
// Which alternative a call means: numeric widening, the most specific parameter type, a
// parameterless alternative, varargs, defaults, by-name parameters, function shapes, generic
// alternatives, a second argument list, the expected result type, named arguments.
object Catalogue:
  def widen(x: Long) = "long"
  def widen(x: Any) = "any"
  def number(x: Long) = "long"
  def number(x: Double) = "double"
  def count: Int = 1
  def count(step: Int): Int = step + 1
  def reset(): Int = 1
  def reset(to: Int): Int = to + 1
  def label(id: Int): String = "id " + id
  def label(title: String): String = "title " + title
  def shelve(ids: Int*) = "many"
  def shelve(id: Int) = "one"
  def lend(id: Int, days: Int = 14) = "with default"
  def lend(id: Int) = "plain"
  def later(due: => Int) = "by name"
  def later(due: Int, grace: Int) = "two"
  def each(f: Int => Int) = "ids"
  def each(f: String => Int, from: Int) = "titles"
  def fold(f: Int => Int) = "unary"
  def fold(f: (Int, Int) => Int) = "binary"
  def find(by: Option[Int]) = "option"
  def find(by: List[Int]) = "list"
  def wrap[T](x: T) = "generic"
  def wrap(x: Int) = "int"
  def move(id: Int)(shelf: Int) = "to shelf " + shelf
  def move(id: Int)(room: String) = "to room " + room
  def price(id: Int): Int = 1
  def price(id: Any): String = "free"
  def tag(id: Int, note: String) = "note"
  def tag(id: Int, colour: Int) = "colour"

@main def run(): Unit =
  println(Catalogue.widen(1))
  println(Catalogue.number(1))
  val n: Int = Catalogue.count
  println(n)
  println(Catalogue.count(1))
  println(Catalogue.reset())
  val f: Int => String = Catalogue.label
  println(f(1))
  println(List("Dune").map(Catalogue.label))
  println(Catalogue.shelve(1))
  println(Catalogue.shelve(1, 2))
  println(Catalogue.shelve())
  println(Catalogue.lend(1))
  println(Catalogue.later(1))
  println(Catalogue.each(x => x + 1))
  println(Catalogue.each((t: String) => t.length, 1))
  println(Catalogue.fold((a, b) => a + b))
  println(Catalogue.fold(a => a))
  println(Catalogue.find(None))
  println(Catalogue.find(Nil))
  println(Catalogue.wrap(1))
  println(Catalogue.wrap("s"))
  println(Catalogue.move(1)(2))
  println(Catalogue.move(1)("east"))
  val p: String = Catalogue.price(1)
  println(p)
  val id = 5
  println(Catalogue.widen(id))
  println(Catalogue.tag(1, colour = 2))
  println(Catalogue.tag(note = "x", id = 2))
