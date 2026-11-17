import scala.language.dynamics
class Shown(val text: String):
  def :=(v: Int): String = text + ":=" + v
  def apply(key: String): Shown = new Shown(text + "(" + key + ")")
class Typed extends Dynamic:
  def selectDynamic(name: String): Shown = new Shown("select " + name)
  def applyDynamic(name: String)(args: Any*): Shown = new Shown("apply " + name)
class Ordered extends Ordering[Int] with Dynamic:
  def compare(x: Int, y: Int): Int = x - y
  def selectDynamic(name: String): String = name
def uses(a: Typed, o: Ordered): Unit =
  println(a.ensuring(true))
  println(a.ensuring(_ != null))
  println(a.formatted("[%s]"))
  println(a → 1)
  a("x") += 1
  println(a.==(a))
  println(a slot (1, 2))
  println(a.slot(using 1))
  println(o.other)
// What teq does not send to a dynamic method, each refused and none a dynamic call.
// `ensuring`, `formatted` and `→` are `scala.Predef`'s for every value, by a conversion scalac tries before the
// dynamic methods, and the lean std does not hold them. `a("x") += 1` is refused by scalac too: the application as
// written finds no `+=` on what the dynamic `apply` gives, and no `update` is made of a dynamic `apply`. A name
// every value has is the member's whatever its form (`a.==(a)` is refused for every class). An infix operation's
// parenthesised list (one tuple under scalac) and a written `using` list (the arguments under scalac) are not
// built as an ordinary call's. On the lean std a class that also extends a library class may lack a member
// scala-library gives it.
// expect: 12:11: error: not supported yet: ensuring, which scala.Predef gives every value, on a value whose class derives from scala.Dynamic
// expect: 13:11: error: not supported yet: ensuring, which scala.Predef gives every value, on a value whose class derives from scala.Dynamic
// expect: 14:11: error: not supported yet: formatted, which scala.Predef gives every value, on a value whose class derives from scala.Dynamic
// expect: 15:11: error: not supported yet: →, which scala.Predef gives every value, on a value whose class derives from scala.Dynamic
// expect: 16:3: error: value += is not a member of Shown
// expect: 17:11: error: value == is not a member of Typed
// expect: 18:11: error: not supported yet: an infix call with a parenthesised argument list on a value whose class derives from scala.Dynamic
// expect: 19:11: error: not supported yet: a `using` argument list on a name sent to a dynamic method
// expect: 20:11: error: not supported yet: other, a name sent to a dynamic method on a value whose class also extends Ordering, on the lean standard library
// expect: 9 errors found
// absent: possible cause
// absent: missing parameter type
// absent: the assignment operator
