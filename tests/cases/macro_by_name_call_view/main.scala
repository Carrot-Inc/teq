// A call's argument for a by-name parameter is the expression in reflection, as in scalac's
// trees, where the typer passes a thunk of it; a macro rebuilding the call with
// `Select.overloaded` gets the thunk once and keeps the type arguments it gives (zio-test's
// `assertTrue(from.getOrElse(none).nonEmpty)`).
package app
import mlib.Calls

object Log:
  var evaluated = 0
  def default: Option[Int] = { evaluated += 1; None }

object Util:
  def pick[A](a: A, b: A): A = b

@main def run(): Unit =
  val from: Either[String, Option[Int]] = Right(Some(1))
  val left: Either[String, Option[Int]] = Left("no")
  println(Calls.describe(from.getOrElse(None)))
  println(Calls.describe(from.getOrElse[Option[Int]](None)))
  println(Calls.rebuild(from.getOrElse(None)).nonEmpty)
  println(Calls.rebuild(from.getOrElse[Option[Int]](Log.default)).nonEmpty)
  println(Log.evaluated)
  println(Calls.rebuild(left.getOrElse(Log.default)).nonEmpty)
  println(Log.evaluated)
  val o: Option[Int] = None
  println(Calls.rebuild(o.getOrElse(3)) + 1)
  println(Calls.widened(Util.pick(1, 2)) match
    case _: Long => "long"
    case _ => "other")
