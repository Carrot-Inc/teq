// A template's argument that is an object is evaluated where the call's arguments are, in their order, wherever
// the template's text writes it: an object whose load runs something (its own statement or val, a superclass's
// statement, an enum's cases, an argument of a base's constructor: a class's default, a secondary constructor's,
// a sealed class's default, a parent trait's default; the outer instance of a base class) is loaded before the
// argument written after it, and loaded where the text does not name it. The expectations are Scala's meaning
// of each call, the arguments left to right: what scalac prints for the same program without the templates.
def note(s: String): Int = { println("  " + s); 1 }
def effect(tag: String): Int = { println("  effect " + tag); 1 }

class ClassDefaultBase(x: Int = note("ClassDefault loaded"))
object ClassDefault extends ClassDefaultBase
class SecondaryBase(x: Int) { def this() = this(note("Secondary loaded")) }
object Secondary extends SecondaryBase
sealed abstract class SealedBase(val n: Int = note("SealedCase loaded"))
case object SealedCase extends SealedBase
trait TraitDefaultBase(x: Int = note("TraitDefault loaded"))
object TraitDefault extends TraitDefaultBase
object Statement { println("  Statement loaded") }
object ValByCall { val v = note("ValByCall loaded") }
class BodyClass { println("  ClassBody loaded") }
object ClassBody extends BodyClass
enum Light { println("  Light case made"); case On, Off }
class WithMember(tag: String) { println("  " + tag + " made"); class Member }
object Keeper { val made = new WithMember("Keeper's instance") }
object Enclosed extends Keeper.made.Member
object LateKeeper { val made = new WithMember("LateKeeper's instance") }
object LateEnclosed extends LateKeeper.made.Member
object Quiet

@js("[$1, $0].length") def swapped(a: Any, b: Any): Int = 2
@js("($1 ? 1 : 0)") def secondAlone(a: Any, b: Boolean): Int = 1

@main def main(): Unit =
  println("the text writes the second argument first: a class's default")
  println(swapped(ClassDefault, effect("b")))
  println("a secondary constructor")
  println(swapped(Secondary, effect("b")))
  println("the text never writes the object: a sealed class's default")
  println(secondAlone(SealedCase, effect("b") == 1))
  println("a parent trait's default, the second argument first")
  println(swapped(TraitDefault, effect("b")))
  println("an object with a statement, never written")
  println(secondAlone(Statement, effect("b") == 1))
  println("a val initialised by a call")
  println(secondAlone(ValByCall, effect("b") == 1))
  println("a superclass's statement")
  println(secondAlone(ClassBody, effect("b") == 1))
  println("an enum's companion: each case runs the class's statement")
  println(secondAlone(Light, effect("b") == 1))
  println("an object made with an instance its parent is nested in, the second argument first")
  println(swapped(Enclosed, effect("b")))
  println("the same, never written")
  println(secondAlone(LateEnclosed, effect("b") == 1))
  println("an object that runs nothing")
  println(swapped(Quiet, effect("b")))
  println("end")
