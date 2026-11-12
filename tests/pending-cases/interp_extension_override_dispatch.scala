// Extension methods of T that U overrides (`^^`, `solo`), called from T's own
// body and through an import of a T, beside the plain method `plain` and its
// override: scalac 3.8.4 prints the .expected, and teq's interpreter is to
// print it too. Found by reviewer #15 of the workers exports helper (push 9).
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
