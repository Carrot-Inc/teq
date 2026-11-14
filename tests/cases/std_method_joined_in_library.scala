// jars: joined-function-lib
// A library's class that is a function and has a trait's `apply` of another parameter beside the function's is
// called as the function and through the trait, by itself and below a program's class that adds a third `apply`;
// the library's classes that do not join stay what they were, and so does the trait's own method where a program's
// class joins it with a program's overload: the lines scalac 3.8.4 prints.
import joinedfn.*

trait Switched { def apply(b: Boolean): Int = 8 }
class Wider extends Scale with Switched

trait Extra { def apply(b: Boolean): Int = 5 }
abstract class Joined extends Chosen with Extra
class Named extends Chosen
class Mixed extends Chosen with Extra

@main def run(): Unit =
  val g: String => Int = new Length
  println(g("four"))
  val c = new Scale
  println((c: String => Int)("two"))
  println((c: Chosen).apply(None))
  val d = new Wider
  println((d: String => Int)("two"))
  println((d: Measure).apply("three"))
  println((d: Chosen).apply(None))
  println(d.apply(true))
  val o: Chosen = new Chosen {}
  println(o.apply(None))
  println(new Named().apply(None))
  val m = new Mixed
  println((m: Chosen).apply(Some(1)))
  println((m: Extra).apply(true))
