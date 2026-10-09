// The scalac side of tests/classpath/jvm/type_test_abi.scala: compiled into a jar by scalac 3.8.4,
// never by teq. teq's `TypeTest` instances, synthesized and handwritten, are handed to it and
// called through scala-library's `scala/reflect/TypeTest` interface, `unapply(Object): Option`.
package tt

import scala.reflect.TypeTest

trait Animal
class Dog(val name: String) extends Animal:
  override def toString = s"Dog($name)"
class Cat extends Animal

object Lib:
  def defined[S, T](x: S)(using test: TypeTest[S, T]): Boolean = test.unapply(x).isDefined
  def viaInterface(test: TypeTest[Any, String], x: Any): Option[String] = test.unapply(x)
  def narrowed(test: TypeTest[Animal, Dog], x: Animal): Option[Dog] = test.unapply(x)
  def onString(test: TypeTest[String, "ok"], x: String): Boolean = test.unapply(x).isDefined
