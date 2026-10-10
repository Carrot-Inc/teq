// A `Conversion` that arrives as a using parameter converts as a given in scope does: applied to
// a value (`x(1)`), selected on (`x.length`), in a method's clause or a class's (dotty's view
// candidates are the context's implicit references, a context parameter's included).
import scala.language.implicitConversions

class C
object G:
  given Conversion[Int, Int => Int] = x => y => x + y

def viaGiven(x: Int) =
  import G.given
  x(1)
def viaUsing(x: Int)(using c: Conversion[Int, Int => Int]) = x(1)
def viaUsingMember(x: C)(using c: Conversion[C, String]) = x.length
def viaUsingCustom(x: C)(using c: Conversion[C, Int => Int]) = x(1)
class Holder(using c: Conversion[C, String]):
  def len(x: C) = x.length

@main def main(): Unit =
  println(viaGiven(2))
  println(viaUsing(2)(using x => y => x * y))
  println(viaUsingMember(new C)(using _ => "abc"))
  println(viaUsingCustom(new C)(using _ => y => y + 10))
  println(Holder(using _ => "four").len(new C))
