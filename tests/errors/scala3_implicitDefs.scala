// From scala/scala3 tests/neg/implicitDefs.scala.
// expect: type of implicit definition needs to be given explicitly
// expect: result type of implicit definition needs to be given explicitly
// expect: recursive value x needs type

object implicitDefs {

  implicit val x = 2 // error: type of implicit definition needs to be given explicitly
  implicit def y(x: Int) = 3 // error: result type of implicit definition needs to be given explicitly
  implicit def z(a: x.type): String = "" // ok

  def foo(implicit x: String) = 1

  def bar() = {
    implicit val x = foo // error: cyclic reference
    x
  }
}
