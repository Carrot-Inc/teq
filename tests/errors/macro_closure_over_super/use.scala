// expect: use.scala:10:27: error: exception in macro: java.lang.Exception: Expected an expression. This is a partially applied Term. Try eta-expanding the term first.
// expect: 1 error found
// `Closure(meth, None)` over `super.f`: `super` names its method where it is written, which a closure that holds
// the receiver's value cannot keep (the call would go to the override), so the reference is no function value
// (scalac's own back end has no tree for it: "Unexpected tree in genLoad: Super").
class A:
  def f(): Int = 1
class B extends A:
  override def f(): Int = 2
  def test(): () => Int = close(super.f())
@main def run(): Unit =
  println(new B().test()())
