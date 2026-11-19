// A protected member reached from a trait whose self type is its class, outside the member's package, needs an
// accessor that no enclosing class can hold (dotty's ProtectedAccessors, `needsAccessor` and `ifNoHost`;
// AccessProxies' `hostForAccessorOf`): the typer lets the trait's `this` reach it, the later phase refuses each
// reference, by identifier, as a value, through `this`, inside a lambda, a `protected[lib]` one, and the implicit
// conversion and the given conversion the search takes through the trait's `this`. scalac refuses each; teq built them.
// expect: 29:25: error: illegal access to protected method m in class C from trait Side
// expect: 30:25: error: illegal access to protected value v in class C from trait Side
// expect: 31:28: error: illegal access to protected method m in class C from trait Side
// expect: 32:43: error: illegal access to protected method m in class C from trait Side
// expect: 33:29: error: illegal access to protected method q in class C from trait Side
// expect: 34:29: error: illegal access to protected method conv in class C from trait Side
// expect: 37:29: error: illegal access to protected given instance conv in class D from trait Given
// expect: 7 errors found
import scala.language.implicitConversions
package lib {
  class Rich(val s: String) { def shout: String = s }
  class C(val origin: String) {
    protected def m(x: Int): String = "m" + x + origin
    protected[lib] def q(x: Int): String = "q" + x + origin
    protected val v: String = "v" + origin
    protected implicit def conv(b: Boolean): Rich = new Rich("conv" + origin)
  }
  class D(val origin: String) {
    protected given conv: Conversion[Int, Rich] = i => new Rich("given" + origin)
  }
}
package use {
  trait Side { self: lib.C =>
    def ident: String = m(1)
    def value: String = v
    def selected: String = this.m(2)
    def lambda: String = List(3).map(i => m(i)).head
    def qualified: String = q(4)
    def converted: String = true.shout
  }
  trait Given { self: lib.D =>
    def converted: String = 5.shout
  }
}
