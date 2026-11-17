import scala.language.dynamics
class Selecting extends Dynamic:
  def selectDynamic(name: String): Int => String = n => s"$name($n)"
class Typed extends Dynamic:
  def applyDynamic(name: String)(args: Any*): String = s"apply($name)"
class Neither extends Dynamic
def uses(s: Selecting, a: Typed, n: Neither): Unit =
  println(s.call(1))
  println(a.field)
  n.slot = 3
  val t: n.Member = ???
  println(1 :: n)
// A name the rewrite would send to a dynamic method the class lacks: the method is what is not a member, with the
// reference's hint (`ErrorReporting`), also for a type selected on a value of such a class.
// expect: 8:11: error: value applyDynamic is not a member of Selecting
// expect: 9:11: error: value selectDynamic is not a member of Typed
// expect: 10:3: error: value updateDynamic is not a member of Neither
// expect: 11:10: error: type Member is not a member of
// expect: 12:11: error: value applyDynamic is not a member of Neither
// expect: possible cause: maybe a wrong Dynamic method signature?
// expect: 5 errors found
