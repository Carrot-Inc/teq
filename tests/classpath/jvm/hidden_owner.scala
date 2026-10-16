// jars: hiddenowner-lib
// The owner a member's reference names is the qualifier's class, as scalac writes it (dotty's
// `BCodeBodyBuilder.genApply`, "using sym.owner may lead to IllegalAccessError"): the members of a
// package-private Java class (hidden.Base, and hidden.Root two levels up) are reached through its
// public subclass hidden.Shown, which a reference naming the declaring class cannot do from another
// package (IllegalAccessError, JVMS 5.4.4). Each site runs apart and prints its failure: a method, one
// two levels up, both overloads, a class's override of an interface default and the default itself
// through the class, a field read and written, through a subclass of the subclass, intersections
// either way round and with the subclass, a refinement, a type parameter bounded by the subclass, a
// generic result, a block's result, an opaque type, a value class's underlying value, a lambda's parameter, an object
// and `this` of a class extending the subclass, `super` calls; the declaring class's statics (the
// inherited ones through the subclass are refused, tests/classpath/hidden_owner_statics.scala);
// `Object`'s members on a trait and a class, a trait's `clone` override reached through a trait
// extending it (JDK-8154587's workaround); a Scala member through a subclass: an accessor, a setter,
// a default getter; `Null` and `Nothing` receivers, compiled alone. tests/classfile.scala compares each
// site's reference with scalac's.
package owners

import hidden.{Defaults, Leaf, Shown, StaticBase}

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

trait Cloner:
  override def clone(): Object = this
trait SubCloner extends Cloner
class Cloned extends SubCloner

trait Plain
class PlainClass extends Plain
class Named:
  override def toString = "named"
class SubNamed extends Named

class Held:
  val v: Int = 3
  var w: Int = 4
  def withDefault(x: Int = 5): Int = x
class SubHeld extends Held

object Calls:
  def id[T](x: T): T = x
  def method(x: Shown): Int = x.method()
  def deep(x: Shown): Int = x.deep()
  def overloadInt(x: Shown): Int = x.overloaded(2)
  def overloadString(x: Shown): Int = x.overloaded("ab")
  def implemented(x: Shown): Int = x.implemented()
  def defaultOnly(x: Shown): Int = x.defaultOnly()
  def defaultInterface(x: Defaults): Int = x.defaultOnly()
  def fieldRead(x: Shown): Int = x.field
  def fieldWrite(x: Shown): Int = { x.field = 6; x.field }
  def leaf(x: Leaf): Int = x.method()
  def leafDeep(x: Leaf): Int = x.deep()
  def intersection(x: Shown & java.io.Serializable): Int = x.method()
  def reversed(x: java.io.Serializable & Shown): Int = x.method()
  def subclass(x: Shown & Leaf): Int = x.method()
  def refinement(x: Shown { type Tag = Int }): Int = x.method()
  def bounded[T <: Shown](x: T): Int = x.method()
  def generic(x: Shown): Int = id(x).method()
  def genericRead(x: Shown): Int = id(x).field
  def genericWrite(x: Shown): Int = { id(x).field = 9; x.field }
  // A block's value is its result's, here an `Object` cast to the qualifier's type before erasure,
  // the declaring class being inaccessible.
  def block(x: Shown): Int = { val k = id(2); id(x) }.method()
  def blockRead(x: Shown): Int = { val k = id(2); id(x) }.field
  def opaque(x: Shown): Int = Opaque.method(Opaque.wrap(x))
  def value(x: Shown): Int = new Value(x).method()
  def lambda(x: Shown): List[Int] = List(x).map(_.method())
  def module(): Int = Module.method()
  def viaThis(): Int = new Sub().viaThis()
  def superMethod(): Int = new Sub().superMethod()
  def superDeep(): Int = new Sub().superDeep()
  def staticMethod(): Int = StaticBase.staticMethod()
  def staticRead(): Int = StaticBase.staticField
  def staticWrite(): Int = { StaticBase.staticField = 15; StaticBase.staticField }
  def cloneCall(x: SubCloner): Boolean = x.clone() eq x
  def plainString(x: Plain): Boolean = x.toString.startsWith("owners.PlainClass@")
  def plainHash(x: Plain): Boolean = x.hashCode == System.identityHashCode(x)
  def plainEquals(x: Plain): Boolean = x.equals(x)
  def plainClass(x: Plain): String = x.getClass.getName
  def named(x: SubNamed): String = x.toString
  def accessor(x: SubHeld): Int = x.v
  def setter(x: SubHeld): Int = { x.w = 7; x.w }
  def default(x: SubHeld): Int = x.withDefault()
  // A bottom receiver names the declaring class (`defn.isBottomClass`): never run.
  def nil(x: Null): String = x.toString
  def bottom(x: Nothing): String = x.toString

  def site(name: String)(f: => Any): Unit =
    val shown = try f.toString catch case e: IllegalAccessError => "IllegalAccessError"
    println(s"$name $shown")

  def main(args: Array[String]): Unit =
    site("method")(method(new Shown()))
    site("deep")(deep(new Shown()))
    site("overloadInt")(overloadInt(new Shown()))
    site("overloadString")(overloadString(new Shown()))
    site("implemented")(implemented(new Shown()))
    site("defaultOnly")(defaultOnly(new Shown()))
    site("defaultInterface")(defaultInterface(new Shown()))
    site("fieldRead")(fieldRead(new Shown()))
    site("fieldWrite")(fieldWrite(new Shown()))
    site("leaf")(leaf(new Leaf()))
    site("leafDeep")(leafDeep(new Leaf()))
    site("intersection")(intersection(new Shown()))
    site("reversed")(reversed(new Shown()))
    site("subclass")(subclass(new Leaf()))
    site("refinement")(refinement(new Shown().asInstanceOf[Shown { type Tag = Int }]))
    site("bounded")(bounded(new Leaf()))
    site("generic")(generic(new Shown()))
    site("genericRead")(genericRead(new Shown()))
    site("genericWrite")(genericWrite(new Shown()))
    site("block")(block(new Shown()))
    site("blockRead")(blockRead(new Shown()))
    site("opaque")(opaque(new Shown()))
    site("value")(value(new Shown()))
    site("lambda")(lambda(new Shown()))
    site("module")(module())
    site("viaThis")(viaThis())
    site("superMethod")(superMethod())
    site("superDeep")(superDeep())
    site("staticMethod")(staticMethod())
    site("staticRead")(staticRead())
    site("staticWrite")(staticWrite())
    site("cloneCall")(cloneCall(new Cloned()))
    site("plainString")(plainString(new PlainClass()))
    site("plainHash")(plainHash(new PlainClass()))
    site("plainEquals")(plainEquals(new PlainClass()))
    site("plainClass")(plainClass(new PlainClass()))
    site("named")(named(new SubNamed()))
    site("accessor")(accessor(new SubHeld()))
    site("setter")(setter(new SubHeld()))
    site("default")(default(new SubHeld()))
