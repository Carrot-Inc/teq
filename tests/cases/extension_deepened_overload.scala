// Extension overloads whose receivers are equally specific are told apart by the argument
// lists that follow, among every applicable alternative, the less specific receivers included
// (dotc deepens the prototype of an ambiguous overload): scalatest's `name should include(..)`
// over its two `String` receivers and its generic ones.
trait Matcher[-T]:
  def apply(t: T): Boolean
trait Factory[-T, C[_]]:
  def matcher[U <: T: C]: Matcher[U]
class Word
class Compile
trait Eq[A]
given Eq[String] = new Eq[String] {}
given Eq[Int] = new Eq[Int] {}

object Words:
  extension [T](left: T)(using tag: String) infix def should(m: Matcher[T]): String = s"$tag matcher ${m(left)}"
  extension [T, C[_]](left: T)(using tag: String) infix def should(f: Factory[T, C])(using c: C[T]): String =
    s"$tag factory ${f.matcher[T](using c)(left)}"
  extension (left: String)(using tag: String) infix def should(w: Word): String = s"$tag word $left"
  extension (left: String)(using tag: String) infix def should(c: Compile): String = s"$tag compile $left"
  def include(s: String): Matcher[String] = (t: String) => t.contains(s)
  def equal(a: Any): Factory[Any, Eq] = new Factory[Any, Eq]:
    def matcher[U <: Any: Eq]: Matcher[U] = (u: U) => u == a
  def word: Word = Word()

object C:
  def k(s: String)(y: Int): Int = 1
  def k(s: String)(y: Boolean): Int = 4
  def k[T](s: T)(y: String): Int = 2
  def k[T](s: T)(y: Char): Int = 5

extension (s: String) def h(y: Int): Int = 1
extension (s: String) def h(y: Boolean): Int = 4
extension [T](s: T) def h(y: String): Int = 2

@main def run(): Unit =
  import Words.*
  given String = "t:"
  val name: String = "Color"
  println(name should include("Col"))
  println(name should include("Size"))
  println(name should equal("Color"))
  println(name should word)
  println(name should Compile())
  println(3 should equal(3))
  println("a".h("b"))
  println("a".h(true))
  println(C.k("a")("b"))
  println(C.k("a")(1))
  println(C.k("a")('c'))
