// jars: scala-library
// The JVM's non-local return is scalac's exception (tests/cases/nonlocal_return_lambda): a `catch` of `Throwable`
// around the closure takes the `NonLocalReturnControl` (JavaScript lets the return pass every `catch`,
// docs/COMPATIBILITY.md); a lazy val's initialiser returns from its def; a closure that runs after its def returned
// throws the control out of no handler of its key.
import scala.annotation.nowarn
@nowarn def caught(xs: List[Int]): String =
  try { xs.foreach(x => return "returned"); "fell" } catch { case e: Throwable => "caught " + e.getClass.getSimpleName }
@nowarn def lazyRet(b: Boolean): Int = { lazy val l: Int = if b then return 7 else 8; l + 1 }
@nowarn def mk(): Int => Int = { val f: Int => Int = x => if x > 0 then return ((y: Int) => y) else x; f }
@main def run() =
  println(caught(List(1)))
  println(lazyRet(true))
  println(lazyRet(false))
  val escaped = mk()
  println(try escaped(5).toString catch { case e: scala.runtime.NonLocalReturnControl[?] => "escaped" })
  println(escaped(-3))
