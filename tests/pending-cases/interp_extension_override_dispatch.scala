// scalac 3.8.4 prints the .expected, and so do teq's Scala.js and JVM builds. teq's interpreter calls T's extension
// methods where U overrides them (`^^`, `solo`), from T's own body and through an import of a T: it prints `L^M UP^Q
// S!` and `X^Y`. The override of the plain method `plain` is dispatched. The same on integration a097de53d9 and on
// master efddb38de3. Found by reviewer #15 of the workers exports helper (push 9).
trait T:
  extension (r: String) def ^^(a: String): String = r.concat("^").concat(a)
  def plain(r: String)(a: String): String = r.concat("^").concat(a)
  extension (r: String) def solo: String = r.concat("!")
  def use(): String = ("L" ^^ "M").concat(" ").concat(plain("P")("Q")).concat(" ").concat("S".solo)

class U extends T:
  extension (r: String) override def ^^(a: String): String = "U".concat(r).concat("^").concat(a)
  override def plain(r: String)(a: String): String = "U".concat(r).concat("^").concat(a)
  extension (r: String) override def solo: String = "U".concat(r)

@main def run(): Unit =
  println(U().use())
  val t: T = U()
  import t.*
  println("X" ^^ "Y")
