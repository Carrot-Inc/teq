// expect: 45:38: error: illegal access to protected method m in class C from trait Side
// expect: 46:30: error: illegal access to protected method m in class C from trait Side
// expect: 47:39: error: illegal access to protected method m in class C from trait Side
// expect: 48:33: error: illegal access to protected given instance g in class C from trait Side
// expect: 49:36: error: illegal access to protected method conv in class C from trait Side
// expect: 50:40: error: illegal access to protected method m in class C from trait Side
// expect: 51:38: error: illegal access to protected method m in class C from trait Side
// expect: 52:35: error: illegal access to protected method m in class C from trait Side
// expect: 53:38: error: illegal access to protected method m in class C from trait Side
// expect: 54:31: error: illegal access to protected method m in class C from trait Side
// expect: 55:38: error: illegal access to protected method m in class C from trait Side
// expect: 56:22: error: illegal access to protected value w in class C from trait Side
// expect: 57:38: error: illegal access to protected method m in class C from trait Side
// expect: 58:51: error: illegal access to protected method m in class C from trait Side
// expect: 60:43: error: illegal access to protected method $lessinit$greater$default$1 in object Dflt
// expect: 15 errors found
// The references tests/cases/protected_accessor_dropped holds, where they reach dotty's ProtectedAccessors phase:
// an inline argument its expansion uses, a taken `inline if` or `summonFrom` case, a quote the macro returns, a
// condition that is not constant, a val of no constant type, a by-value argument, a `match` (FirstTransform folds
// no match), a protected constructor's default argument the expansion keeps: each refused, as scalac refuses it.
package lib {
  import scala.language.implicitConversions
  class Rich { def foo = 7 }
  class C {
    protected def m: Int = 3
    protected given g: Int = 3
    protected implicit def conv(x: Boolean): Rich = new Rich
    protected val w: Int = 7
  }
}
package store {
  class Dflt protected (val a: Int = 1)
}
package use {
  object Ops {
    inline def keep(inline x: Int): Int = x
    inline def force(inline x: () => Int): Int = x()
    inline def hold(using inline x: Int): Int = x
    inline def choose(inline b: Boolean, inline x: Int): Int = inline if b then x else 7
    inline def summoned(inline x: Int): Int = scala.compiletime.summonFrom { case _: DummyImplicit => x; case _ => 7 }
    inline def byValue(x: Int): Int = 7
  }
  object Flags { final val on = true }
  trait Side { self: lib.C =>
    def constant: Int = if true then m else 7
    def used: Int = Ops.keep(m)
    def lambda: Int = Ops.force(() => m)
    def viaGiven: Int = Ops.hold
    def conversion: Int = Ops.keep(true.foo)
    def branch: Int = Ops.choose(true, m)
    def summoned: Int = Ops.summoned(m)
    def quoted: Int = Macros.keep(m)
    def flag: Int = if Flags.on then m else 7
    def and: Int = if true && m > 0 then 7 else 1
    def or: Int = { val b = false || m > 0; if b then 7 else 1 }
    def value: Int = w
    def evaluated: Int = Ops.byValue(m)
    def matched: Int = false match { case true => m; case false => 7 }
  }
  object Defaults { def f: Int = Ops.keep(new store.Dflt() { }.a) }
}
