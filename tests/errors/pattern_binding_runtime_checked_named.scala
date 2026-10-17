// teq: --werror
// The exemption is `scala.annotation.internal.RuntimeChecked`'s, recognised by the class the
// annotation resolves to (dotty's `defn.RuntimeCheckedAnnot`, which `Checking.checkIrrefutable`
// tests), not by its name: a class of the program named `RuntimeChecked` exempts nothing, on an
// ascription or on a method's result (scalac's message names the type with the user's annotation).
class RuntimeChecked extends scala.annotation.StaticAnnotation
def checked(o: Option[Int]): Option[Int] @RuntimeChecked = o
def f(): Unit =
  val Some(a) = (Option(7): @RuntimeChecked)
  val Some(b) = (Option(8): Option[Int] @RuntimeChecked)
  val Some(c) = checked(Option(9))
  ()
// expect: 9:18: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 10:18: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 11:17: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
