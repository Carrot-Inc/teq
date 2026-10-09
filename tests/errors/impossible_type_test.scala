// expect: 44:40: error: unreachable case: type C1 and class C2 are unrelated
// expect: 45:40: error: unreachable case: type C1 and class C2 are unrelated
// expect: 46:45: error: unreachable case: type F1 is not a subclass of trait T2
// expect: 47:44: error: unreachable case: type C1 and class C2 are unrelated
// expect: 48:54: error: unreachable case: type C1 | C3 and class C2 are unrelated
// expect: 49:44: error: unreachable case: type C1 and class C2 are unrelated
// expect: 50:54: error: unreachable case: type C1 and class C2 are unrelated
// expect: 51:40: error: unreachable case: type Throwable and class C2 are unrelated
// expect: 52:60: error: unreachable case: type C1 and class C2 are unrelated
// expect: 53:44: error: unreachable case: type String is not a subclass of class Integer
// expect: 54:63: error: unreachable case: type C1 and class C2 are unrelated
// expect: 44:40: warning: unreachable case
// expect: 62:47: warning: the type test for Box[String] cannot be checked at runtime
// expect: 66:66: error: unreachable case: type C1 and class C2 are unrelated
// expect: 67:72: error: unreachable case: type Throwable and class C2 are unrelated
// absent: 55:40: error
// absent: 56:44: error
// absent: 57:45: error
// absent: 58:45: error
// absent: 59:27: error
// absent: 60:30: error
// absent: 61:54: error
// absent: 62:47: error
// absent: 63:61: error
// absent: 64:59: error
// absent: 65:76: error
// A type test no value of the scrutinee can pass is an error, as scalac 3.8.4's erasure phase
// reports it (E030, "Unreachable case"), beside the space engine's warning: a class unrelated to
// a class, a final class and a trait, an extractor's input, each part of a union scrutinee, an
// intersection's class, a nested pattern, a catch, a partial function, an `@unchecked` match, a
// case after a type test that always passes. scalac's check leaves a trait a subclass may mix in,
// a union's parts, a primitive scrutinee, an inline method's tests, `isInstanceOf` (a warning
// there), an `Any` scrutinee, an erased type argument and the cases after a wildcard or a binder,
// which its pattern matcher drops, without error; and it reports nothing where the typer reported
// an error (tests/errors/impossible_type_test_typer.scala).
class C1
class C2
class C3
trait T2
final class F1
class Box[A]
object TakesC2:
  def unapply(c: C2): Option[Int] = Some(1)
def typed(x: C1): Int = x match { case _: C2 => 1; case _ => 0 }
def bound(x: C1): Int = x match { case c: C2 => 1; case _ => 0 }
def finalTrait(x: F1): Int = x match { case _: T2 => 1; case _ => 0 }
def extractor(x: C1): Int = x match { case TakesC2(n) => n; case _ => 0 }
def unionScrutinee(x: C1 | C3): Int = x match { case _: C2 => 1; case _ => 0 }
def interTest(x: C1): Int = x match { case _: (C2 & T2) => 1; case _ => 0 }
def nested(x: Option[C1]): Int = x match { case Some(_: C2) => 1; case _ => 0 }
def caught(): Int = try 0 catch { case _: C2 => 1 }
def collected(xs: List[C1]): List[Int] = xs.collect { case _: C2 => 1 }
def boxed(x: String): Int = x match { case _: Integer => 1; case _ => 0 }
def uncheckedFirst(x: C1): Int = (x: @unchecked) match { case _: C2 => 1; case _ => 0 }
def mixed(x: C1): Int = x match { case _: T2 => 1; case _ => 0 }
def unionTest(x: C1): Int = x match { case _: (C2 | C3) => 1; case _ => 0 }
def primitive(x: Int): Int = x match { case _: String => 1; case _ => 0 }
inline def inl(x: C1): Int = x match { case _: C2 => 1; case _ => 0 }
def inlined(x: C1): Int = inl(x)
def direct(x: C1): Boolean = x.isInstanceOf[C2]
def anyScrutinee(x: C1): Int = (x: Any) match { case _: C2 => 1; case _ => 0 }
def erased(x: Box[Int]): Int = x match { case _: Box[String] => 1; case _ => 0 }
def afterWildcard(x: C1): Int = x match { case _ => 0; case _: C2 => 1 }
def afterBinder(x: C1): Int = x match { case y => 0; case TakesC2(n) => n }
def afterUnchecked(x: C1): Int = (x: @unchecked) match { case _ => 1; case _: C2 => 2 }
def afterTypeOfAll(x: C1): Int = x match { case _: C1 => 0; case _: C2 => 1 }
def caughtAfterAll(): Int = try 0 catch { case t: Throwable => 1; case _: C2 => 2 }
@main def run(): Unit = println(typed(C1()))
