// A given read through the instance or object its class or trait is a member of is that
// receiver's reference, dotty's `TermRef(receiver, member)`: lookup, conformance and the
// argument's tree all keep the receiver (an implicit object of a class's instance, of an object
// nested in an object that extends the trait, a given object and an alias given of a singleton
// type, a nested class's companion, an object's own givens through its path); of two lexical
// givens that are one value the first is taken (dotty's `disambiguate`, `alt1.ref =:= alt2.ref`),
// and of one given read on two objects the one whose object is the more specific
// (`Applications.compare`'s `comparePrefixes`).
package instance:
  trait A:
    implicit object U:
      def name = "instance"
    def check(using u: U.type): String = "ok " + u.name
    inline def icheck(inline x: Boolean)(using u: U.type): String = "ok " + x + " " + u.name
  class B extends A
  def show(): Unit =
    val a = new B
    val b = new B
    val alias: b.type = b
    println(a.check + " / " + a.icheck(true))
    println((summon[a.U.type] eq a.U, summon[b.U.type] eq b.U, a.U eq b.U, summon[alias.U.type] eq b.U))

package deep:
  trait A:
    object N:
      implicit object U:
        def name = "deep"
      def check(using u: U.type): String = u.name
      inline def icheck(using u: U.type): String = u.name
  object B extends A
  def show(): Unit =
    println(B.N.check + " / " + B.N.icheck)
    import B.N.*
    println(check + " " + (summon[B.N.U.type] eq B.N.U))

package givenobject:
  trait A:
    trait Token:
      def name: String
    given u: Token with
      def name = "given object"
    def check(using x: u.type): String = x.name
    inline def icheck(using x: u.type): String = x.name
  object B extends A
  def show(): Unit =
    import B.*
    println(check + " / " + icheck)

package givenalias:
  trait A:
    object U:
      def name = "alias given"
    given u: U.type = U
    def check(using x: U.type): String = x.name
  object B extends A
  def show(): Unit = println(B.check)

package selfscope:
  trait A:
    object U:
      given me: U.type = U
      def name = "own given"
    def check(using u: U.type): String = u.name
  object B extends A
  def show(): Unit = println(B.check)

package prefixscope:
  class Foo(val n: Int)
  trait A:
    val x: Foo = Foo(3)
  object B extends A:
    given bx: x.type = x
  def show(): Unit = println(summon[B.x.type].n)

package twoobjects:
  trait A:
    implicit object U:
      def name = "B's"
    def check(using u: U.type): String = u.name
  object B extends A
  object C extends A
  def show(): Unit =
    import B.*
    import C.U as CU
    println((check, summon[B.U.type] eq B.U, summon[C.U.type] eq C.U, B.U eq C.U))

package companions:
  trait A:
    class C(val n: Int)
    object C:
      given c: C = new C(7)
    trait Instances:
      given d: D = new D(8)
    class D(val n: Int)
    object D extends Instances
    object N:
      class E(val n: Int)
      object E:
        given e: E = new E(9)
  object B extends A
  def show(): Unit = println((summon[B.C].n, summon[B.D].n, summon[B.N.E].n))

package enums:
  trait A:
    enum E:
      case One, Two
      case Three(n: Int)
    object E:
      given one: One.type = One
      given three: Three = Three(3)
  object B extends A
  def show(): Unit = println((summon[B.E.One.type], summon[B.E.Three]))

package stable:
  class Foo(val n: Int)
  given x: Foo = Foo(1)
  given y: x.type = x
  object O:
    given z: Foo = Foo(2)
  def show(): Unit = println((summon[x.type].n, summon[O.z.type].n, summon[Foo].n))

package lexical:
  trait A:
    implicit object U
    def chosen: Boolean =
      given x: U.type = null.asInstanceOf[U.type]
      summon[U.type] eq x
    def same: Boolean =
      given x: U.type = U
      given y: U.type = U
      summon[U.type] eq U
  object B extends A
  def show(): Unit = println((B.chosen, B.same))

package prefixes:
  class I[X](val tag: String)
  trait A:
    def name: String = "A"
    given I[B] = I[B](name)
  object A extends A
  trait B extends A:
    override def name: String = "B"
  object B extends B
  def show(): Unit = println(summon[I[B]].tag)

package extprefixes:
  trait OpsT:
    extension (c: C) def who: String
  trait A:
    def name: String = "A"
    given ops: OpsT with
      extension (c: C) def who: String = name
  trait B extends A:
    override def name: String = "B"
  class P
  object P extends B
  class C extends P
  object C extends A
  def show(): Unit = println(C().who)

@main def run(): Unit =
  instance.show()
  deep.show()
  givenobject.show()
  givenalias.show()
  selfscope.show()
  prefixscope.show()
  twoobjects.show()
  companions.show()
  enums.show()
  stable.show()
  lexical.show()
  prefixes.show()
  extprefixes.show()
