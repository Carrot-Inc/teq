package loa

// scalac 3.8.4's inline rules, through a product: a member of the parameter is
// resolved at its declared type (`B.value`, so `pick`'s overload of `B`) and the call names the
// override of the argument's class (`O$.value()` on the JVM); a nested expansion ranks at the
// inner parameter's declared type.
trait B:
  def value: B
object O extends B:
  def value: O.type = this
class C extends B:
  def value: B = this
def pick(x: Any): String = "any"
def pick(x: B): String = "base"
def pick(x: O.type): String = "object"
def pick(x: C): String = "class"
inline def member(x: B): String = pick(x.value)
inline def inner(x: Any): String = pick(x)
inline def outer(x: B): String = inner(x)
