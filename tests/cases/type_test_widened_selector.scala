// A selector that is a reference is widened before its patterns are typed, a constant type
// staying only where the selector's own type is one (dotty's `typedMatch`): `case _: T` over
// a parameter `x: 1` searches a `TypeTest[Int, T]`, over the literal `1` a `TypeTest[1, T]`.
import scala.reflect.TypeTest

var calls = 0

given one: TypeTest[1, String] with
  def unapply(x: 1): Option[x.type & String] = { calls += 1; None }
given two: TypeTest[2, String] with
  def unapply(x: 2): Option[x.type & String] = { calls += 1; None }

final val K = 1
val V: 1 = 1
def single(): 1 = 1

def param[T](x: 1)(using tt: TypeTest[1, T]): Boolean = x match
  case _: T @unchecked => true
  case _ => false
def literal[T](using tt: TypeTest[1, T]): Boolean = 1 match
  case _: T @unchecked => true
  case _ => false
def constant[T](using tt: TypeTest[1, T]): Boolean = K match
  case _: T @unchecked => true
  case _ => false
def value[T](using tt: TypeTest[1, T]): Boolean = V match
  case _: T @unchecked => true
  case _ => false
def ascribed[T](using tt: TypeTest[1, T]): Boolean = (1: 1) match
  case _: T @unchecked => true
  case _ => false
def folded[T](using tt: TypeTest[2, T]): Boolean = (1 + 1) match
  case _: T @unchecked => true
  case _ => false
def call[T](using tt: TypeTest[1, T]): Boolean = single() match
  case _: T @unchecked => true
  case _ => false
def union[T](x: 1 | 2)(using tt: TypeTest[1 | 2, T]): Boolean = x match
  case _: T @unchecked => true
  case _ => false

def show(name: String, r: Boolean): Unit =
  println(s"$name $r $calls")
  calls = 0

@main def run() =
  show("param", param[String](1))
  show("literal", literal[String])
  show("constant", constant[String])
  show("value", value[String])
  show("ascribed", ascribed[String])
  show("folded", folded[String])
  show("call", call[String])
  given TypeTest[1 | 2, String] = new TypeTest[1 | 2, String]:
    def unapply(x: 1 | 2): Option[x.type & String] = { calls += 1; None }
  show("union", union[String](2))
