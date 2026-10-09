// The scalac side of abi_callbacks.scala: compiled into a jar by scalac 3.8.4, never by teq.
package abi

// Compiled by scalac into a jar; teq's classes are handed to it and called back.
object Lib:
  def applyTwice(f: Int => Int, x: Int): Int = f(f(x))
  def applyBoth(f: (Int, Int) => Int): Int = f(3, 4)
  def compose(f: Int => Int, g: Int => Int): Int => Int = f.andThen(g)
  def describe(p: Product): String =
    s"${p.productPrefix} ${p.productArity} ${p.productIterator.toList} ${p.productElementNames.toList} ${p.canEqual(p)}"
  def names(p: Product): String = p.productElementNames.mkString(",")
  def canEq(p: Product, q: Any): Boolean = p.canEqual(q)
  def hash(a: Any): Int = a.##
  def sameAs(a: Any, b: Any): Boolean = a == b && a.## == b.##
  def viaCompanion(className: String, arg: Int): String =
    val module = Class.forName(className + "$").getField("MODULE$").get(null)
    val apply = module.getClass.getMethod("apply", classOf[Int], classOf[Int])
    val made = apply.invoke(module, Int.box(arg), Int.box(arg + 1))
    val unapply = module.getClass.getMethod("unapply", made.getClass)
    s"$made ${unapply.invoke(module, made)}"
  def viaStaticForwarder(className: String): String =
    Class.forName(className).getMethod("greeting").invoke(null).toString
  def runAll(tasks: Seq[() => String]): String = tasks.map(_()).mkString(",")
  def sortDesc(xs: Seq[Int], ord: Ordering[Int]): Seq[Int] = xs.sorted(ord.reverse)

// Reflection over the program's classes, as Java code and frameworks find them.
object Reflect:
  private def moduleOf(className: String): AnyRef = Class.forName(className + "$").getField("MODULE$").get(null)
  private def boxed(args: Seq[Any]): Seq[AnyRef] = args.map(_.asInstanceOf[AnyRef])
  def companion(className: String, args: Any*): String =
    val module = moduleOf(className)
    val apply = module.getClass.getMethods.filter(_.getName == "apply").head
    val made = apply.invoke(module, boxed(args)*)
    val unapply = module.getClass.getMethod("unapply", made.getClass)
    val mirror = module.asInstanceOf[scala.deriving.Mirror.Product]
    s"$made ${unapply.invoke(module, made)} ${mirror.fromProduct(made.asInstanceOf[Product])}"
  def moduleName(className: String): String = moduleOf(className).toString
  def finalModule(className: String): Boolean =
    java.lang.reflect.Modifier.isFinal(Class.forName(className + "$").getField("MODULE$").getModifiers)
  def constructorDefault(className: String, n: Int): Any =
    val module = moduleOf(className)
    module.getClass.getMethod("$lessinit$greater$default$" + n).invoke(module)
  def copy(value: AnyRef, args: Any*): Any =
    value.getClass.getMethods.filter(_.getName == "copy").head.invoke(value, boxed(args)*)
  def copyDefault(value: AnyRef, n: Int): Any = value.getClass.getMethod("copy$default$" + n).invoke(value)
  def static(className: String, method: String, args: Any*): Any =
    val m = Class.forName(className).getMethods.filter(m => m.getName == method && java.lang.reflect.Modifier.isStatic(m.getModifiers)).head
    m.invoke(null, boxed(args)*)
  def values(className: String): String =
    Class.forName(className).getMethod("values").invoke(null).asInstanceOf[Array[AnyRef]].mkString(",")
  def valueOf(className: String, name: String): Any =
    Class.forName(className).getMethod("valueOf", classOf[String]).invoke(null, name)
  def fromOrdinal(className: String, n: Int): Any =
    Class.forName(className).getMethod("fromOrdinal", classOf[Int]).invoke(null, Int.box(n))
  def javaEnum(e: java.lang.Enum[?]): String = s"${e.name} ${e.ordinal} ${e.getDeclaringClass.getSimpleName}"
  def runMain(className: String, args: String*): Unit =
    Class.forName(className).getMethod("main", classOf[Array[String]]).invoke(null, args.toArray)

// A jar enum whose values the program reads.
enum Level:
  case Low, High
  case Custom(n: Int)

// Stackable traits of a jar: a program class mixing them in implements the super accessors.
trait Base:
  def m: Int = 1
trait Doubling extends Base:
  override def m: Int = super.m * 2
trait Incrementing extends Base:
  override def m: Int = super.m + 10

object ReflectValue:
  def extension(className: String, method: String, args: Any*): Any =
    val module = Class.forName(className + "$").getField("MODULE$").get(null)
    val m = module.getClass.getMethods.filter(_.getName == method + "$extension").head
    m.invoke(module, args.map(_.asInstanceOf[AnyRef])*)

object ReflectModule:
  private def moduleOf(className: String): AnyRef = Class.forName(className + "$").getField("MODULE$").get(null)
  def call(className: String, method: String, args: Any*): Any =
    val module = moduleOf(className)
    val m = module.getClass.getMethods.filter(m => m.getName == method && m.getParameterCount == args.size).head
    m.invoke(module, args.map(_.asInstanceOf[AnyRef])*)
  def overloads(className: String, method: String): String =
    moduleOf(className).getClass.getMethods.filter(_.getName == method).map(_.getParameterTypes.map(_.getSimpleName).mkString("(", ",", ")")).sorted.mkString(" ")
  def staticField(className: String, field: String): Any = Class.forName(className).getField(field).get(null)
  def privateConstructor(className: String): Boolean =
    Class.forName(className).getDeclaredConstructors.forall(c => java.lang.reflect.Modifier.isPrivate(c.getModifiers))

// A jar superclass with a final canEqual, and a jar abstract class stacking traits.
abstract class FinalEquals extends Product:
  final override def canEqual(that: Any): Boolean = that.isInstanceOf[FinalEquals]
abstract class Stacked extends Doubling with Incrementing

object ReflectStatics:
  def count(className: String, method: String): Int =
    Class.forName(className).getMethods.count(m => m.getName == method && java.lang.reflect.Modifier.isStatic(m.getModifiers))

// Defaults of by-name parameters: scalac's getters return the value, and a call leaving the
// argument out passes a thunk that calls the getter each time the parameter is used.
object Defaults:
  var evaluated = 0
  def clue(x: Int, msg: => String = { evaluated += 1; "none" }): String = s"$x $msg $msg"
  def twice(f: => Int = 21): Int = f + f
class Tagged(val tag: String):
  def label(n: Int)(suffix: => String = tag + n): String = s"$n-$suffix"
