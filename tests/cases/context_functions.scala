// Context function types: `A ?=> B` as a type, an expression wrapped where one is expected
// (its parameter a given in the body), a context function literal, a value of the type applied
// to the givens in scope where its result is expected, an explicit `(using ...)` application,
// and a context function returning a function.
trait Ctx:
  def tag: String
  extension (s: String) def tagged: String = s + "#" + tag

object Main:
  def run[A](f: Ctx ?=> A): A = f(using new Ctx { def tag = "run" })
  def twice(f: Ctx ?=> String): String = run(f) + run(f)
  val greeting: Ctx ?=> String = "hi".tagged
  def uses(using c: Ctx): String = c.tag
  def main(args: Array[String]): Unit =
    println(run(summon[Ctx].tag))
    println(run("x".tagged))
    println(twice("y".tagged))
    println(run(greeting))
    val g: Ctx ?=> Int = (c: Ctx) ?=> c.tag.length
    println(run(g))
    val h: (Int, Ctx) ?=> String = (n: Int, c: Ctx) ?=> c.tag * n
    given Ctx = new Ctx { def tag = "given" }
    given Int = 2
    println(h)
    println(greeting)
    println(run(uses))
    def apply2(f: Ctx ?=> Int => Int): Int = f(using summon[Ctx])(3)
    println(apply2(n => n + summon[Ctx].tag.length))
