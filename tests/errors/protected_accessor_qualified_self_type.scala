// expect: 16:47: error: illegal access to protected given instance g in class Impl from trait Left
// expect: 17:51: error: illegal access to protected method m in class B from trait Right
// expect: 2 errors found
// A member qualified `protected[lib]`, the compiler's own (a deferred given's implementation) or written, read
// through a self type from outside lib: no enclosing class can hold the accessor, and each reference is refused as
// scalac refuses it. tests/modules/protectedqualifiedrefused refuses the same over teq's products, and
// tests/cases/protected_accessor_made_members runs the members where the qualifier holds the site.
package lib.inner {
  trait Q { protected[lib] given g: Int = scala.compiletime.deferred }
  class B { protected[lib] def m: Int = 6 }
}
package use {
  class Impl(using n: Int) extends lib.inner.Q
}
package outside {
  trait Left { self: use.Impl => def f: Int = g }
  trait Right { self: lib.inner.B => def h: Int = m }
}
