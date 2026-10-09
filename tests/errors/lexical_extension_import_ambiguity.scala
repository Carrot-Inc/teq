// expect: 20:13: error: ambiguous extension methods: both B.pick and A.pick are possible expansions of pick on R
// expect: 21:13: error: ambiguous extension methods: both B.same and A.same are possible expansions of same on R
// expect: 2 errors found
// Two imports of one scope whose extensions both take the receiver are an ambiguity, whatever
// their explicit parameters (`pick(x: String)` beside `pick(x: Int)`: the prefix `pick(r)` is
// all that is tried), reported where the implicit scope has no extension of the name either
// (scalac: "Ambiguous extension methods: both B.pick(new R()) and A.pick(new R()) are possible
// expansions").
class R
object A:
  extension (r: R) def pick(x: Int): String = "A"
  extension (r: R) def same(x: String): String = "A"
object B:
  extension (r: R) def pick(x: Int): String = "B"
  extension (r: R) def same(x: Int): String = "B"
object Main:
  import A.*
  import B.*
  def main(args: Array[String]): Unit =
    println((new R).pick(1))
    println((new R).same(1))
