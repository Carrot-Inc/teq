// The calls of tests/classpath/jvm/hidden_owner.scala over Scala classes, which the interpreter and
// JavaScript run as the JVM does (the Java jar's program the interpreter refuses, having no body of a
// Java class to run, tests/classfile.scala): a member a superclass two levels up declares, an overload,
// a class's override of a trait's default and the default itself, a field read and written, through a
// subclass of the subclass, intersections, a refinement, a bounded type parameter, a generic result, a
// block's result, an ascription, a cast that keeps no node (an upcast, in a program and in an inline
// body, and a tested value's), an inline call's result, an opaque type, a value class's underlying
// value, a lambda's parameter, an object and `this` of a class extending the subclass, `super` calls,
// an object's members, `Object`'s members on a trait, a trait's `clone` reached through a trait
// extending it, an accessor, a setter and a default getter through a subclass. On the JVM each
// reference names the qualifier's class as scalac's does (tests/classfile.scala compares them).
package hiddenscala:
  abstract class Root:
    final def deep(): Int = 11

  trait Defaults:
    def defaultOnly(): Int = 31
    def implemented(): Int = 30

  abstract class Base extends Root with Defaults with Serializable:
    var field: Int = 5
    final def method(): Int = 7
    final def overloaded(x: Int): Int = x + 10
    final def overloaded(x: String): Int = x.length + 20
    override final def implemented(): Int = 32

  class Shown extends Base
  class Leaf extends Shown

  object Statics:
    var field: Int = 8
    def method(): Int = 9

package hiddenclient:
  import hiddenscala.{Defaults, Leaf, Shown, Statics}

  object Opaque:
    opaque type Wrapped = Shown
    def wrap(x: Shown): Wrapped = x
    def method(x: Wrapped): Int = x.method()

  class Value(val underlying: Shown) extends AnyVal:
    def method(): Int = underlying.method()

  class Sub extends Shown:
    def superMethod(): Int = super.method()
    def superDeep(): Int = super.deep()
    def viaThis(): Int = method() + this.deep()

  object Module extends Shown

  object Makers:
    inline def make(): Defaults = new Defaults {}
    inline def leaf(): Leaf = new Leaf()
    inline def shown[A](a: A): Int = a.asInstanceOf[Shown].method()

  trait Cloner:
    override def clone(): Object = this
  trait SubCloner extends Cloner
  class Cloned extends SubCloner

  trait Plain
  class PlainClass extends Plain:
    override def toString = "plain"
    override def hashCode = 17
  class Named:
    override def toString = "named"
  class SubNamed extends Named

  class Held:
    val v: Int = 3
    var w: Int = 4
    def withDefault(x: Int = 5): Int = x
  class SubHeld extends Held

  object Main:
    def id[T](x: T): T = x
    def method(x: Shown): Int = x.method()
    def deep(x: Shown): Int = x.deep()
    def overloadInt(x: Shown): Int = x.overloaded(2)
    def overloadString(x: Shown): Int = x.overloaded("ab")
    def implemented(x: Shown): Int = x.implemented()
    def defaultOnly(x: Shown): Int = x.defaultOnly()
    def defaultTrait(x: Defaults): Int = x.defaultOnly()
    def fieldRead(x: Shown): Int = x.field
    def fieldWrite(x: Shown): Int = { x.field = 6; x.field }
    def leaf(x: Leaf): Int = x.method()
    def leafDeep(x: Leaf): Int = x.deep()
    def intersection(x: Shown & Serializable): Int = x.method()
    def subclass(x: Shown & Leaf): Int = x.method()
    def refinement(x: Shown { type Tag = Int }): Int = x.method()
    def bounded[T <: Shown](x: T): Int = x.method()
    def generic(x: Shown): Int = id(x).method()
    def genericRead(x: Shown): Int = id(x).field
    // A block's value is its result's: an `Object` cast to the declaring class, a local of the
    // subclass named on it (`Typer.typedBlock`, then `Erasure.typedSelect`).
    def block(x: Shown): Int = { val k = id(2); id(x) }.method()
    def blockLocal(x: Shown): Int = { val y = x; y }.method()
    def blockRead(x: Shown): Int = { val k = id(2); id(x) }.field
    // An ascription's value is of the ascribed type, an inline call's result of the method's declared
    // one: `Typed`'s erasure, though the node is a subclass's.
    def ascribed(x: Leaf): Int = (x: Shown).method()
    def ascribedNew(): Int = (new Leaf(): Shown).method()
    def inlined(): Int = Makers.make().defaultOnly()
    def inlinedLeaf(): Int = Makers.leaf().method()
    // The ascribed type stands whatever the node's own erasure (a generic result's `Object`, cast
    // to it), the outermost ascription deciding.
    def genericAscribed(x: Leaf): Int = (id(x): Shown).method()
    def nestedAscribed(x: Leaf): Int = ({ val k = id(1); (x: Shown) }: hiddenscala.Base).method()
    // A cast's value is of its target: an upcast is an ascription (`TypeTestsCasts.transformAsInstanceOf`),
    // an inline body's where its type argument is known, and a tested value's cast is cast.
    def upcast(x: Leaf): Int = x.asInstanceOf[Shown].method()
    def inlineUpcast(x: Leaf): Int = Makers.shown(x)
    def tested(x: Shown): Int = if x.isInstanceOf[Leaf] then x.asInstanceOf[Leaf].method() else 0
    def opaque(x: Shown): Int = Opaque.method(Opaque.wrap(x))
    def value(x: Shown): Int = new Value(x).method()
    def lambda(x: Shown): List[Int] = List(x).map(_.method())
    def module(): Int = Module.method()
    def statics(): Int = { Statics.field = 15; Statics.method() + Statics.field }
    def cloneCall(x: SubCloner): Boolean = x.clone() eq x
    def plainString(x: Plain): String = x.toString
    def plainHash(x: Plain): Int = x.hashCode
    def plainEquals(x: Plain): Boolean = x.equals(x)
    def named(x: SubNamed): String = x.toString
    def accessor(x: SubHeld): Int = x.v
    def setter(x: SubHeld): Int = { x.w = 7; x.w }
    def default(x: SubHeld): Int = x.withDefault()

    def main(args: Array[String]): Unit =
      val sub = new Sub()
      println(List(method(new Shown()), deep(new Shown()), overloadInt(new Shown()), overloadString(new Shown()), implemented(new Shown())))
      println(List(defaultOnly(new Shown()), defaultTrait(new Shown()), fieldRead(new Shown()), fieldWrite(new Shown())))
      println(List(leaf(new Leaf()), leafDeep(new Leaf()), intersection(new Shown()), subclass(new Leaf()), bounded(new Leaf())))
      println(List(refinement(new Shown().asInstanceOf[Shown { type Tag = Int }]), generic(new Shown()), genericRead(new Shown()), opaque(new Shown()), value(new Shown())))
      println(List(block(new Shown()), blockLocal(new Shown()), blockRead(new Shown())))
      println(List(ascribed(new Leaf()), ascribedNew(), inlined(), inlinedLeaf(), genericAscribed(new Leaf()), nestedAscribed(new Leaf()), upcast(new Leaf()), inlineUpcast(new Leaf()), tested(new Leaf())))
      println(lambda(new Shown()))
      println(List(module(), sub.viaThis(), sub.superMethod(), sub.superDeep(), statics()))
      println(List(cloneCall(new Cloned()), plainString(new PlainClass()), plainHash(new PlainClass()), plainEquals(new PlainClass()), named(new SubNamed())))
      println(List(accessor(new SubHeld()), setter(new SubHeld()), default(new SubHeld())))
