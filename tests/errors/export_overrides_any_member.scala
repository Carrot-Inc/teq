// expect: B.this.a.toString cannot be exported because it overrides method toString in class Any, which is already a member of class B
// expect: C.this.a.hashCode cannot be exported because it overrides method hashCode in class Any, which is already a member of class C
// expect: D.this.a.equals cannot be exported because it overrides method equals in class Any, which is already a member of class D
// expect: E.this.a.foo cannot be exported because it overrides method foo in class Base, which is already a member of class E
// A member that overrides a concrete one the exporting class has already is not forwarded: `Any`'s
// members are concrete members of every class (dotty's `Namer.canForward` over
// `allOverriddenSymbols`, `neg/i7086`); `bar` implements an abstract one and is forwarded.
class Base:
  def foo: Int = 1
  def bar: Int
class A extends Base:
  override def toString: String = "A"
  override def hashCode: Int = 7
  override def equals(that: Any): Boolean = false
  override def foo = 2
  def bar = 2
class B(val a: A):
  export a.toString
class C(val a: A):
  export a.hashCode
class D(val a: A):
  export a.equals
class E(val a: A) extends Base:
  export a.foo
  export a.bar
