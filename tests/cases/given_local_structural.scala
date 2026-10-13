// A local structural given without parameters is a local object (dotty's `Parsers.givenDef`, a
// `ModuleDef` where the given has no parameters): made on its first use in each run of the block,
// its body's members and extensions read through the search; `given QC()` of a trait is one too.
trait T { def n: Int }
trait Show[A] { def show(a: A): String }
trait QC { def q = "qc" }
case class Ctx(name: String)
object M:
  given QC()
  given Ctx("alpha")
@main def run(): Unit =
  var made = 0
  given T with { made += 1; def n = 42 }
  println(made)
  println(summon[T].n)
  println(summon[T].n + made)
  given Show[Int] with
    def show(a: Int) = s"<$a>"
    extension (i: Int) def shown = show(i)
  println(summon[Show[Int]].show(1))
  println(2.shown)
  def f(k: Int): Int =
    given T with { def n = k }
    summon[T].n
  println(f(1) + f(2))
  locally {
    import M.given
    println(summon[QC].q + " " + summon[Ctx] + " " + (summon[Ctx] == Ctx("alpha")))
  }
  locally {
    given QC()
    println(summon[QC].q)
  }
