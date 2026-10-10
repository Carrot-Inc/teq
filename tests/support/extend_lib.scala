// The jar side of tests/classpath/js/extend_lib, compiled by scalac: a class extending
// PartialFunction, one extending ClassTag with overrides, and members whose results are
// scala-library's views.
package extendlib

class Offset(val offset: Int) extends PartialFunction[Int, Int]:
  def isDefinedAt(x: Int): Boolean = { println(s"defined $x"); x > 0 }
  def apply(x: Int): Int = x + offset

abstract class Base extends PartialFunction[String, Int]:
  def isDefinedAt(x: String): Boolean = x.nonEmpty
  def apply(x: String): Int = x.length

class Tag extends scala.reflect.ClassTag[String]:
  def runtimeClass: Class[?] = classOf[String]
  override def unapply(x: Any): Option[String] = { println(s"unapply $x"); Some(s"custom $x") }
  override def newArray(n: Int): Array[String] = { println(s"newArray $n"); Array.fill(n)("custom") }

object Views:
  def seq: scala.collection.SeqView[Int] = List(1, 2, 3).view
  def indexed: scala.collection.IndexedSeqView[Int] = Vector(1, 2, 3).view
  def mapped(v: scala.collection.SeqView[Int]): scala.collection.SeqView[Int] = v.map(_ * 10)
  def filtered(v: scala.collection.SeqView[Int]): scala.collection.View[Int] = v.filter(_ > 1)
