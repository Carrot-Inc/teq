// jars: scala-library traitfields-lib
// A trait's givens in the class mixing it in, of a jar's trait (`tfl.Showing`) and the program's alike: one
// without parameters is a lazy val, which the class holds (made once, its getter final as scalac's); one with a
// type parameter, a given class or an alias, is scalac's def, which the class does not hold, so every call makes
// one (the whole build held it, and `pair[Int] eq pair[Int]` was true where scalac's is false); an alias's
// lambda is left out of the identities, which the JVM caches for scalac's non-capturing lambda alone.
import tfl.{Show, Showing}

trait PShowing:
  given pplain: Show[Int] = { println("make pplain"); a => "pplain " + a }
  given ppair[A]: Show[A] with
    def show(a: A): String = "{" + a + "}"
  given palias[A]: Show[List[A]] = { println("make palias"); as => as.mkString("(", ",", ")") }

object J extends Showing
object P extends PShowing
class JC extends Showing

@main def run(): Unit =
  println((J.plain.show(1), J.plain eq J.plain))
  println((J.pair[Int].show(2), J.pair[Int] eq J.pair[Int]))
  println(J.alias[Int].show(List(3, 4)) + J.alias[Int].show(Nil))
  println((P.pplain.show(1), P.pplain eq P.pplain))
  println((P.ppair[Int].show(2), P.ppair[Int] eq P.ppair[Int]))
  println(P.palias[Int].show(List(3, 4)) + P.palias[Int].show(Nil))
  val c = new JC
  println(c.plain eq c.plain)
