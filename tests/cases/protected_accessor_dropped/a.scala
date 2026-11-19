// A protected member's reference that does not reach dotty's ProtectedAccessors phase needs no accessor, and is not
// refused (tests/errors/protected_accessor_kept refuses the same references where they reach it):
// - the inlining phase drops an inline argument its expansion does not use (a value, a lambda, a given, a
//   conversion), an untaken `inline if`, an unselected `summonFrom` case and a quote a macro's expansion leaves out,
//   and expands a call of a protected inline method in its place;
// - FirstTransform folds a constant condition (`false`, a `final val`, `false && x`, `true || x`, in an `if` or
//   not) and reads a constant val through a pure path as its literal.
// A protected constructor's default argument dropped the same way is not refused either.
package lib {
  import scala.language.implicitConversions
  class Rich { def foo = 7 }
  class C {
    protected def m: Int = 3
    protected given g: Int = 3
    protected implicit def conv(x: Boolean): Rich = new Rich
    protected final val k = 7
    protected inline def mi: Int = 7
  }
}
package store {
  class Dflt protected (val a: Int = 1)
}
package use {
  object Ops {
    inline def ignore(inline x: Int): Int = 7
    inline def discard(inline x: () => Int): Int = 7
    inline def skip(using inline x: Int): Int = 7
    inline def choose(inline b: Boolean, inline x: Int): Int = inline if b then x else 7
    inline def summoned(inline x: Int): Int = scala.compiletime.summonFrom { case _: String => x; case _ => 7 }
  }
  object Flags { final val off = false }
  trait Side { self: lib.C =>
    def constant: Int = if false then m else 7
    def unused: Int = Ops.ignore(m)
    def lambda: Int = Ops.discard(() => m)
    def viaGiven: Int = Ops.skip
    def conversion: Int = Ops.ignore(true.foo)
    def branch: Int = Ops.choose(false, m)
    def summoned: Int = Ops.summoned(m)
    def quoted: Int = Macros.ignore(m)
    def flag: Int = if Flags.off then m else 7
    def and: Int = if false && m > 0 then 1 else 7
    def or: Int = { val b = true || m > 0; if b then 7 else 1 }
    def literal: Int = k
    def inlined: Int = mi
  }
  class Sub extends lib.C with Side
  object Defaults { def f: Int = Ops.ignore(new store.Dflt() { }.a) + (if false then new store.Dflt() { }.a else 0) }
  @main def run(): Unit =
    val s = new Sub
    println(List(s.constant, s.unused, s.lambda, s.viaGiven, s.conversion, s.branch, s.summoned).mkString(" "))
    println(List(s.quoted, s.flag, s.and, s.or, s.literal, s.inlined, Defaults.f).mkString(" "))
}
