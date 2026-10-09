// jars: scala-library cats-kernel cats-core
// Under --target jvm the reach pass converts more of cats' bodies than under JavaScript and
// stops in `cats.Invariant.catsInvariantMonoid` and its neighbours with `type mismatch: found
// Monoid[B], required AnyRef & Monoid[B]`; the same program builds and runs for JavaScript
// (`Some(List(1, 2, 3))`). This keeps the corpus's API side from the JVM target (tests/app.sh
// notes it) while its JS build passes.
import cats.syntax.all.*
object Main:
  def main(args: Array[String]): Unit =
    println(List(1, 2, 3).traverse(x => Option(x)))
