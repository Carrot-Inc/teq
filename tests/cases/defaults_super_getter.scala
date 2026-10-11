// A getter's application on `super` is the superclass's or the trait's getter, as
// `Applications.findDefaultGetter` selects it on the receiver, not the override's.
def mark(s: String): Int = { print(s); 1 }
class A { def f(a: Int = mark("A"))(b: Int): Int = a + b; def g(a: Int = mark("a"), b: Int = mark("b")): Int = a + b }
class B extends A {
  override def f(a: Int = mark("X"))(b: Int): Int = a + b
  override def g(a: Int = mark("x"), b: Int = mark("y")): Int = a + b
  def go(): Int = super.f()(mark("B"))
  def go2(): Int = super.g(b = mark("B"))
  def go3(): Int = { val h = () => super.f()(mark("C")); h() }
  def go4(): Int = f()(mark("D"))
}
trait T { def t(a: Int = mark("T"))(b: Int): Int = a + b }
class U extends T { override def t(a: Int = mark("V"))(b: Int): Int = a + b; def go(): Int = super[T].t()(mark("U")) }
@main def main(): Unit = {
  val b = new B
  println(b.go())
  println(b.go2())
  println(b.go3())
  println(b.go4())
  println(new U().go())
}
