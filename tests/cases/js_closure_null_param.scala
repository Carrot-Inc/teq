//> using platform js
//> using jsVersion 1.22.0
// A closure of a trait extending js.Function receives its parameters from JavaScript as any and unboxes each to its
// type (JSCodeGen#genClosure, `unbox`): a `Null` parameter is tested to be null, a ClassCastException otherwise
// (.expected from scalac 3.8.4 -scalajs).
import scala.scalajs.js

trait F extends js.Function:
  def apply(x: Null): Int

@main def main(): Unit =
  val f: F = (_: Null) => 42
  println(f.asInstanceOf[js.Function1[Null, Int]](null))
  try println(f.asInstanceOf[js.Function1[String, Int]]("bad"))
  catch case _: ClassCastException => println("ClassCastException")
