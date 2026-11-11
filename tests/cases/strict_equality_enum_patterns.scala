//> using options -language:strictEquality
// teq: --strict-equality
import scala.language.strictEquality
import scala.language.experimental.strictEqualityPatternMatching
enum Shade:
  case Pale, Dark
enum Signal[+A]:
  case Empty extends Signal[Nothing]
  case Text extends Signal[String]
  case Value(value: A)
object Check:
  def same(value: Shade): Int = value match
    case Shade.Pale => 1
    case Shade.Dark => 2
  def bottom(value: Signal[Int]): Int = value match
    case Signal.Empty => 1
    case _ => 2
  def wildcard(value: Signal[?]): Int = value match
    case Signal.Text => 1
    case _ => 2

@main def run(): Unit =
  println(Check.same(Shade.Pale))
  println(Check.same(Shade.Dark))
  println(Check.bottom(Signal.Empty))
  println(Check.bottom(Signal.Value(3)))
  println(Check.wildcard(Signal.Text))
  println(Check.wildcard(Signal.Value(true)))
