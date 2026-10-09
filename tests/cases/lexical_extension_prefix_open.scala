// A using clause before the method's parameters whose type the receiver leaves open is searched by
// the prefix as it stands, as dotty's `adaptNoArgs` searches it: an instance fixes the variable for
// the application (`lexical int`), none or two fail the prefix and the companion's or the other
// import's extension is taken (`companion`, `B`), where counting the open clause as found selected
// the lexical candidate or made the imports an ambiguity. What a search fixes holds for the
// prefix's next clause: `Need[Int]` makes `Closed[T]` a `Closed[Int]`, one of two `Closed` givens
// (`int closed int`).
trait Need[A]
trait Show[A]:
  def name: String
class R
object R:
  extension (r: R) def pick(x: Int): String = "companion pick"
  extension (r: R) def show(x: Int): String = "companion show"
  extension (r: R) def amb(x: Int): String = "companion amb"
object A:
  extension [T](r: R)(using Need[T]) def take(x: T): String = "A"
object B:
  extension (r: R) def take(x: Int): String = "B"
object One:
  given Show[Int] with
    def name = "int"
  extension [T](r: R)(using s: Show[T]) def show(x: T): String = "lexical " + s.name
  def run(): String = (new R).show(1)
object Two:
  given Show[Int] with
    def name = "int"
  given Show[String] with
    def name = "string"
  extension [T](r: R)(using s: Show[T]) def amb(x: T): String = "lexical " + s.name
  def run(): String = (new R).amb(1)
trait Labelled[A]:
  def label: String
trait Closed[A]:
  def label: String
class Q
object Q:
  extension (q: Q) def chain(x: Int): String = "companion chain"
object Chain:
  given Labelled[Int] with
    def label = "int"
  given Closed[Int] with
    def label = "closed int"
  given Closed[String] with
    def label = "closed string"
  extension [T](q: Q)(using n: Labelled[T])(using c: Closed[T])
    def chain(x: T): String = n.label + " " + c.label
  def run(): String = (new Q).chain(1)
object Main:
  extension [T](r: R)(using Need[T]) def pick(x: T): String = "lexical pick"
  def main(args: Array[String]): Unit =
    println((new R).pick(1))
    println(One.run())
    println(Two.run())
    println(Chain.run())
    locally {
      import A.*
      import B.*
      println((new R).take(1))
    }
