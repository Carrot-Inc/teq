//> using platform js
//> using jsVersion 1.21.0
// interp-expected: js
// Scala.js's reflective instantiation: classes and objects that carry
// @EnableReflectiveInstantiation, or inherit it, are found by name at run time.
package reflecttest

import scala.scalajs.reflect.Reflect
import scala.scalajs.reflect.annotation.EnableReflectiveInstantiation

@EnableReflectiveInstantiation
trait Plugin:
  def name: String

@EnableReflectiveInstantiation
class Unboxed(val z: Point, val i: Int, val l: Long, val d: Double, val b: Boolean, val c: Char, val s: String):
  override def toString = s"$z $i $l ${d.toInt} $b ${c.toInt} $s"

@EnableReflectiveInstantiation
class Point(val x: Int, val y: String):
  def this(x: Int) = this(x, "one")
  private def this() = this(0, "private")
  override def toString = s"Point($x, $y)"

class Hello extends Plugin:
  def name = "hello"

class Shapes(d: Double, l: Long, b: Boolean, c: Char, s: String, p: Point, h: Hello, f: Int => Int) extends Plugin:
  def name = "shapes"

class Generic[T](val t: T) extends Plugin:
  def name = s"generic $t"

class Curried(a: Int)(b: String)(using c: Double) extends Plugin:
  def name = s"curried $a $b $c"

class Defaults(a: Int = 3, b: String = "b") extends Plugin:
  def name = s"defaults $a $b"

class Guarded protected (a: Int) extends Plugin:
  def name = "guarded"
  def this(s: String) = this(s.length)

class Hidden private (a: Int) extends Plugin:
  def name = "hidden"

class Qualified private[reflecttest] (a: Int) extends Plugin:
  def name = "qualified"

class QualifiedProtected protected[reflecttest] (a: Int) extends Plugin:
  def name = "qualified protected"

class QualifiedSecondaries(a: Int) extends Plugin:
  private[reflecttest] def this() = this(1)
  protected[reflecttest] def this(s: String) = this(2)
  def name = "qualified secondaries"

@EnableReflectiveInstantiation
enum Shade:
  case Tint(level: Int)
  case Plain

class Outer(val k: Int):
  class In(val v: Int) extends Plugin:
    def name = s"in ${v + k}"
  trait Mixed:
    class Deep(val w: Int) extends Plugin:
      def name = s"deep ${w + k}"
  def mixed: Mixed = new Mixed {}

class Meters(val value: Double) extends AnyVal
class Measured(val m: Meters) extends Plugin:
  def name = s"measured ${m.value.toInt}"

class Animal
trait Pet
class Erasures(a: => Int, m: Meters, p: Animal & Pet, q: Pet & Animal, f: () => Int) extends Plugin:
  def name = "erasures"

class Kin
class Child extends Kin
trait Tagged
class Grandchild extends Child with Tagged
class Family(k: Kin, c: Child, g: Grandchild, t: Tagged) extends Plugin:
  def name = "family"

class Unitary(u: Unit) extends Plugin:
  def name = "unitary"

class Varargs(a: Int, xs: Int*) extends Plugin:
  def name = s"varargs $a ${xs.sum}"

abstract class Base extends Plugin

case class Record(a: Int, b: String) extends Plugin:
  def name = s"record $a $b"

class Plain(val a: Int)

class Multi(a: Int, b: Int) extends Plugin:
  def name = s"multi $a $b"
  def this(s: String) = this(s.length, 0)
  def this() = this(1, 2)
  def this(x: Double, y: Double) = this(x.toInt, y.toInt)

@EnableReflectiveInstantiation
object Registry:
  val count = 42
  object Inner extends Plugin:
    def name = "inner"
  class Nested(val v: Int) extends Plugin:
    def name = s"nested $v"

object Loner extends Plugin:
  def name = "loner"

def describe(fqcn: String): Unit =
  Reflect.lookupInstantiatableClass(fqcn) match
    case None => println(s"$fqcn: none")
    case Some(c) =>
      println(s"$fqcn: ${c.runtimeClass.getName}")
      for ctor <- c.declaredConstructors do
        println("  " + ctor.parameterTypes.map(_.getName).mkString("(", ", ", ")"))

def module(fqcn: String): Unit =
  Reflect.lookupLoadableModuleClass(fqcn) match
    case None => println(s"$fqcn: no module")
    case Some(m) =>
      val v = m.loadModule()
      println(s"$fqcn: ${m.runtimeClass.getName} ${v match { case p: Plugin => p.name; case r => r == Registry }}")

@main def main(): Unit =
  for n <- List("reflecttest.Point", "reflecttest.Hello", "reflecttest.Shapes", "reflecttest.Generic",
      "reflecttest.Curried", "reflecttest.Defaults", "reflecttest.Guarded", "reflecttest.Hidden",
      "reflecttest.Base", "reflecttest.Plugin", "reflecttest.Record", "reflecttest.Plain", "reflecttest.Multi", "reflecttest.Qualified", "reflecttest.QualifiedProtected", "reflecttest.QualifiedSecondaries", "reflecttest.Shade$Tint", "reflecttest.Shade", "reflecttest.Outer$In", "reflecttest.Outer$Mixed$Deep", "reflecttest.Erasures", "reflecttest.Unitary",
      "reflecttest.Registry$Nested", "reflecttest.Registry", "reflecttest.Registry$", "reflecttest.Nope") do
    describe(n)
  for n <- List("reflecttest.Registry$", "reflecttest.Registry", "reflecttest.Registry$Inner$",
      "reflecttest.Loner$", "reflecttest.Point", "reflecttest.Record$") do
    module(n)
  val point = Reflect.lookupInstantiatableClass("reflecttest.Point").get
  println(point.getConstructor(classOf[Int]).map(_.newInstance(7)))
  println(point.getConstructor(classOf[Int], classOf[String]).map(_.newInstance(8, "eight")))
  println(point.getConstructor(classOf[String]))
  try point.newInstance()
  catch case e: InstantiationException => println(s"${e.getClass.getName}: ${e.getMessage} ${e.getCause}")
  println(Reflect.lookupInstantiatableClass("reflecttest.Hello").get.newInstance().asInstanceOf[Plugin].name)
  println(Reflect.lookupInstantiatableClass("reflecttest.Record").get.declaredConstructors.head.newInstance(1, "r"))
  println(Reflect.lookupInstantiatableClass("reflecttest.Registry$Nested").get.declaredConstructors.head.newInstance(5).asInstanceOf[Plugin].name)
  try Reflect.lookupInstantiatableClass("reflecttest.Shapes").get.newInstance()
  catch case e: InstantiationException => println(s"${e.getClass.getName}: ${e.getMessage}")
  try point.getConstructor(classOf[Int]).get.newInstance(1, 2)
  catch case e: IllegalArgumentException => println(s"${e.getClass.getName}: ${e.getMessage}")
  val curried = Reflect.lookupInstantiatableClass("reflecttest.Curried").get.declaredConstructors.head
  println(curried.newInstance(1, "two", 3.5).asInstanceOf[Plugin].name)
  val defaults = Reflect.lookupInstantiatableClass("reflecttest.Defaults").get.declaredConstructors.head
  println(defaults.newInstance(4, "d").asInstanceOf[Plugin].name)
  println(Registry.count)
  println(Reflect.lookupInstantiatableClass("reflecttest.Unboxed").get.declaredConstructors.head.newInstance(null, null, null, null, null, null, null))
  val family = Reflect.lookupInstantiatableClass("reflecttest.Family").get.declaredConstructors.head.parameterTypes
  println(for a <- family yield family.map(b => if a.isAssignableFrom(b) then 1 else 0).mkString)
  val pointClass = Reflect.lookupInstantiatableClass("reflecttest.Point").get.runtimeClass
  println(List(family.head, pointClass, classOf[Int], classOf[String]).map(classOf[AnyRef].isAssignableFrom))
  val measured = Reflect.lookupInstantiatableClass("reflecttest.Measured").get.declaredConstructors.head
  println(measured.parameterTypes.map(_.getName))
  println(measured.newInstance(2.0).asInstanceOf[Plugin].name + " " + measured.newInstance(null).asInstanceOf[Plugin].name)
  val varargs = Reflect.lookupInstantiatableClass("reflecttest.Varargs").get
  println(varargs.getConstructor(classOf[Int], classOf[Seq[?]]).map(_.newInstance(1, Seq(2, 3)).asInstanceOf[Plugin].name))
  println(Reflect.lookupInstantiatableClass("reflecttest.Shade$Tint").get.declaredConstructors.head.newInstance(4))
  println(Reflect.lookupLoadableModuleClass("reflecttest.Shade$").isDefined)
  val outer = Outer(10)
  println(Reflect.lookupInstantiatableClass("reflecttest.Outer$In").get.declaredConstructors.head.newInstance(outer, 5).asInstanceOf[Plugin].name)
  val mixed = outer.mixed
  println(Reflect.lookupInstantiatableClass("reflecttest.Outer$Mixed$Deep").get.declaredConstructors.head.newInstance(mixed, 1).asInstanceOf[Plugin].name)
