// Imports of one scope that each bring an extension of the name are alternatives of dotty's
// `findRef` (`altImports`), each tried alone by its prefix: one that applies is selected, two
// are an ambiguity, which, like none, leaves the selection to the implicit scope. A named
// import stands in the place of the wildcards; an inner scope's import shadows an outer one's.
class R
class S
trait Need
object R:
  extension (r: R) def amb(x: Int): String = "companion amb"
  extension (r: R) def nogiven: String = "companion nogiven"

object A:
  extension (r: R) def amb(x: Int): String = "A amb"
  extension (s: S) def one(x: Int): String = "A one"
  extension (r: R) def named(x: Int): String = "A named"
  extension (r: R) def nogiven(using Need): String = "A nogiven"
  extension (r: R) def over(x: Int): String = "A over Int"
  extension (r: R) def over(x: String): String = "A over String"
  extension (r: R) def inner: String = "A inner"
object B:
  extension (r: R) def amb(x: Int): String = "B amb"
  extension (r: R) def one(x: Int): String = "B one"
  extension (r: R) def named(x: Int): String = "B named"
  extension (r: R) def nogiven: String = "B nogiven"
  extension (r: R) def inner: String = "B inner"

object Main:
  import A.*
  import B.{named as _, *}
  import A.named

  def main(args: Array[String]): Unit =
    val r = new R
    println(r.amb(1))
    println(r.one(2))
    println(r.named(3))
    println(r.nogiven)
    println(r.over("s"))
    println(r.over(4))
    locally {
      import B.inner
      println(r.inner)
    }
