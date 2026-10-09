// expect: 34:43: error: unreachable case: type O and class C2 are unrelated
// expect: 35:50: error: unreachable case: type X and class C2 are unrelated
// expect: 38:42: error: unreachable case: type HasMember.this.M and class C2 are unrelated
// expect: 39:46: error: unreachable case: type Color and class String are unrelated
// expect: 40:53: error: unreachable case: type Single is not a subclass of class C2
// expect: 41:40: error: unreachable case: type C1 and class Tuple2 are unrelated
// expect: 42:44: error: unreachable case: type F is not a subclass of trait T
// expect: 43:44: error: unreachable case: type T and class F are unrelated
// absent: 44:45: error
// absent: 45:57: error
// absent: 46:53: error
// absent: 47:52: error
// absent: 48:48: error
// absent: 49:58: error
// absent: 52:65: error
// absent: 53:63: error
// absent: 56:42: error
// absent: 57:57: error
// The impossible type test's error over the classes scalac 3.8.4 judges: an opaque type, a
// bounded type parameter and type member, an enum, an object, a tuple class, a final class
// against a trait either way; none for a value class, where the classes' hierarchy is the JDK's
// (`String` is a `CharSequence`, `Integer` and arrays are `Serializable`), nor over an abstract
// type whose bound is an intersection of unrelated classes (either order) or a union.
class C1
class C2
trait T
final class F
object Op:
  opaque type O = C1
class V(val i: Int) extends AnyVal
enum Color:
  case Red, Green
object Single
def opaque(x: Op.O): Int = x match { case _: C2 => 1; case _ => 0 }
def bounded[X <: C1](x: X): Int = x match { case _: C2 => 1; case _ => 0 }
trait HasMember:
  type M <: C1
  def member(x: M): Int = x match { case _: C2 => 1; case _ => 0 }
def enumCase(x: Color): Int = x match { case _: String => 1; case _ => 0 }
def singleton(x: Single.type): Int = x match { case _: C2 => 1; case _ => 0 }
def tuple(x: C1): Int = x match { case _: (Int, Int) => 1; case _ => 0 }
def finalTrait(x: F): Int = x match { case _: T => 1; case _ => 0 }
def traitFinal(x: T): Int = x match { case _: F => 1; case _ => 0 }
def valueClass(x: C1): Int = x match { case _: V => 1; case _ => 0 }
def charSequence(x: CharSequence): Int = x match { case _: String => 1; case _ => 0 }
def stringSequence(x: String): Int = x match { case _: CharSequence => 1; case _ => 0 }
def serializable(x: Integer): Int = x match { case _: java.io.Serializable => 1; case _ => 0 }
def array(x: Array[Int]): Int = x match { case _: java.io.Serializable => 1; case _ => 0 }
def comparable(x: Comparable[Int]): Int = x match { case _: Integer => 1; case _ => 0 }
class Parent
trait Mark
def boundParent[X <: Parent & Mark](x: X): Int = x match { case _: C2 => 1; case _ => 0 }
def boundMark[X <: Mark & Parent](x: X): Int = x match { case _: C2 => 1; case _ => 0 }
trait HasBoth:
  type B <: Parent & Mark
  def member(x: B): Int = x match { case _: C2 => 1; case _ => 0 }
def boundUnion[X <: C1 | F](x: X): Int = x match { case _: C2 => 1; case _ => 0 }
