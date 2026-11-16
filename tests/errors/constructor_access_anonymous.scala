// expect: 51:14: error: constructor Bin cannot be accessed as a member of Bin from object Side
// expect: 52:22: error: constructor Bin cannot be accessed as a member of Bin from class S2
// expect: 55:20: error: constructor Bin cannot be accessed as a member of Bin from class A1
// expect: 56:20: error: constructor Bin cannot be accessed as a member of Bin from class A2
// expect: 57:21: error: constructor Bin cannot be accessed as a member of Bin from object O1
// expect: 58:20: error: constructor Bin cannot be accessed as a member of Bin from trait T1
// expect: 59:20: error: constructor Box cannot be accessed as a member of Box from trait T2
// expect: 60:20: error: constructor Bin cannot be accessed as a member of Bin from class A3
// expect: 61:9: error: class V3 cannot extend final class Val
// expect: 63:14: error: constructor Bin cannot be accessed as a member of Bin from object Use
// expect: 64:14: error: constructor Bin cannot be accessed as a member of Bin from object Use
// expect: 65:24: error: constructor Bin cannot be accessed as a member of Bin from object Use
// expect: 66:19: error: constructor Bin cannot be accessed as a member of Bin from object Use
// expect: 67:14: error: constructor Box cannot be accessed as a member of Box from object Use
// expect: 68:14: error: anonymous class {...} cannot extend final class Fin
// expect: 69:14: error: anonymous class {...} cannot extend final class FinP
// expect: 70:14: error: anonymous class {...} cannot extend final class Val
// expect: 71:14: error: anonymous class {...} cannot extend final class ValP
// expect: 72:14: error: illegal inheritance: self type Self {...} of anonymous class Self {...} does not conform to self type Need of parent class Self
// expect: 73:14: error: illegal inheritance: self type SelfP {...} of anonymous class SelfP {...} does not conform to self type Need of parent class SelfP
// expect: 75:22: error: constructor Bin cannot be accessed as a member of Bin from object Use
// expect: 76:18: error: constructor Bin cannot be accessed as a member of Bin from object Use
// expect: 77:14: error: constructor FinG cannot be accessed as a member of FinG from object Use
// expect: 23 errors found
// An anonymous class of an empty body is its parent's creation, and what a subclass is asked of its parent is
// asked of it (tests/cases/constructor_access_anonymous_parent holds what stays right). scalac's lines beside each:
// - a parent whose type as written leaves type parameters is typed ahead of the class's parents, with or without
//   an argument list, for a class, an object, a trait and an anonymous class alike: no super call, so a protected
//   constructor is none of its alternatives outside another subclass (E173 "constructor Bin cannot be accessed as
//   a member of store.Bin from class A1"); an expected type fixes a bare parent's arguments, never `Any`;
// - a final class and a value class are extended by no class (E093 "anonymous class {...} cannot extend final
//   class Fin"; "class V3 cannot extend final class Val");
// - the anonymous class has the parent's self type to conform to (E058 "illegal inheritance: self type store.Self
//   {...} of anonymous class store.Self {...} does not conform to self type store.Need of parent class Self"): the
//   two lines of a later phase of scalac's (RefChecks), which it tells once the typer's twenty-one are gone.
// One error a line, as scalac gives one: an expected type that fixes none of a bare parent's parameters (a
// wildcard, a variable still open) leaves it typed ahead; a final class out of reach is told of its constructor
// alone. A protected constructor's default getter is tests/errors/constructor_access_default_getter.scala.
package store:
  class Bin[T] protected () { def v: Int = 7 }
  class Box[T] protected (val x: T)
  final class Fin(val x: Int)
  final class FinP protected (val x: Int)
  class Val(val n: Int) extends AnyVal
  class ValP protected (val n: Int) extends AnyVal
  trait Need
  class Self(val x: Int) { self: Need => }
  class SelfP protected (val x: Int) { self: Need => }
  final class FinG[T] protected (val x: T)
  object Side:
    val s1 = new Bin {}
    class S2 extends Bin
package shop:
  import store.*
  class A1 extends Bin
  class A2 extends Bin()
  object O1 extends Bin
  trait T1 extends Bin
  trait T2 extends Box(1)
  class A3 extends Bin with Serializable
  class V3 extends Val(1)
  object Use:
    val u1 = new Bin {}
    val u2 = new Bin() {}
    val u3: Bin[Int] = new Bin() {}
    val u4: Any = new Bin {}
    val u5 = new Box(1) {}
    val f1 = new Fin(1) {}
    val f2 = new FinP(1) {}
    val v1 = new Val(1) {}
    val v2 = new ValP(1) {}
    val s1 = new Self(1) {}
    val s2 = new SelfP(1) {}
    def gen[A](b: Bin[A]): Int = b.v
    val w1: Bin[?] = new Bin {}
    val w2 = gen(new Bin {})
    val g1 = new FinG(1) {}
