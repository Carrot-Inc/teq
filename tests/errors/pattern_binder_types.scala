// expect: 30:63: error: type mismatch: found B & A, required String
// expect: 31:68: error: type mismatch: found B & C & A, required String
// expect: 32:67: error: type mismatch: found B & A | B & C, required String
// expect: 33:73: error: type mismatch: found B2 | B & C, required String
// expect: 34:77: error: type mismatch: found B & A | B2, required String
// expect: 36:76: error: type mismatch: found Box[T$1], required Box[Any]
// expect: 37:75: error: type mismatch: found T$1, required String
// expect: 38:78: error: type mismatch: found Box[T$1], required Box[AnyVal]
// expect: 45:80: error: value foo is not a member of
// expect: 46:67: error: type mismatch: found B, required Int
// absent: 35:53: error
// absent: 39:73: error
// absent: 40:55: error
// absent: 43:81: error
// The binders scalac 3.8.4 gives patterns, by its own messages: a type pattern's binder the
// pattern's type and then the scrutinee's (`B & A`), each part of a union scrutinee its own (`B2`
// conforms, `B & C` not; `Integer` kept, which is `Serializable` though the lean std does not
// say so; a part below another absorbed, `B & A | B` is `B`); a generic extractor's parameter
// the scrutinee leaves open at an invariant position a fresh abstract type within its bounds
// (`T$1`), not its upper bound, unless that is final.
trait A { def a: Int = 1 }
trait B { def b: Int = 2 }
trait B2 extends B
class C
class Box[T](val v: T)
object Ex:
  def unapply[T](b: Box[T]): Some[T] = Some(b.v)
object ExU:
  def unapply[T <: AnyVal](b: Box[T]): Some[T] = Some(b.v)
def typed(x: A): Int = x match { case b: B => val s: String = b; 0; case _ => 1 }
def both(x: A): Int = x match { case b: (B & C) => val s: String = b; 0; case _ => 1 }
def union(x: A | C): Int = x match { case b: B => val s: String = b; 0; case _ => 1 }
def conforming(x: B2 | C): Int = x match { case b: B => val s: String = b; 0; case _ => 1 }
def conformingLast(x: A | B2): Int = x match { case b: B => val s: String = b; 0; case _ => 1 }
def members(x: A | C): Int = x match { case b: B => b.b; case _ => 1 }
def invariant(x: Any): Int = x match { case b @ Ex(v) => val y: Box[Any] = b; 0; case _ => 1 }
def invariantValue(x: Any): Int = x match { case Ex(v) => val s: String = v; 0; case _ => 1 }
def bounded(x: Any): Int = x match { case b @ ExU(v) => val y: Box[AnyVal] = b; 0; case _ => 1 }
def wildcard(x: Any): Int = x match { case b @ Ex(v) => val y: Box[?] = b; val w: Any = v; 0; case _ => 1 }
def fixed(x: Box[Int]): Int = x match { case Ex(v) => v + 1 }
object ExS:
  def unapply[T <: String](b: Box[T]): Some[T] = Some(b.v)
def finalBound(x: Any): Int = x match { case b @ ExS(v) => val y: Box[String] = b; v.length; case _ => 0 }
trait S extends java.io.Serializable { def foo: String = "S" }
def serial(x: Integer | S): String = x match { case b: java.io.Serializable => b.foo; case _ => "other" }
def absorbed(x: A | B): Int = x match { case b: B => val s: Int = b; 0; case _ => 1 }
