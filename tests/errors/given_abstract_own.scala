// A concrete class's own member without a body is unimplemented as an inherited one is, an old-style
// abstract given among them (scalac's "needs to be abstract"); an abstract class and a trait keep
// theirs. teq accepted the class, which then failed at run time. What the member overrides, a
// concrete one included, implements nothing of it.
// expect: 10:7: error: class C needs to be abstract, since given def x: Int in class C is not defined
// expect: 11:7: error: class D needs to be abstract, since def y: Int in class D is not defined
// expect: 12:7: error: class E needs to be abstract, since val z: String in class E is not defined
// expect: 16:7: error: class H needs to be abstract, since override def x: Int in class H is not defined
// expect: 17:7: error: class I needs to be abstract, since override given def x: Int in class I is not defined
class C { given x: Int }
class D { def y: Int }
class E { val z: String }
abstract class F { given x: Int; def y: Int }
trait G { given x: Int }
trait Concrete { def x: Int = 41 }
class H extends Concrete { override def x: Int }
class I extends Concrete { override given x: Int }
