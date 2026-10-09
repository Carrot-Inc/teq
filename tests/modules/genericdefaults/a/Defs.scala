package gda

// Default getters whose parameter's type names the method's type parameters: scalac's getter has the default's
// type (`a$default$1()String`), teq's the parameter's in the pickle and the class file alike, which a scalac
// downstream calls as the pickle says.
object Defs:
  def a[A](x: A = "hi"): A = x
  def c[A](x: List[A] = Nil): Int = x.size
  def d[A](x: A)(y: List[A] = List(x, x)): Int = y.size
  def e[A](x: Option[A] = None): Int = x.size
class Box[T]:
  def n[A](x: A = 3.5): A = x
