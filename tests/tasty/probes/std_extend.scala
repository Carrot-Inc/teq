package probes.stdextend

import scala.collection.{View, SeqView, IndexedSeqView}
import scala.reflect.ClassTag

// The products of classes extending scala-library's PartialFunction and ClassTag, with their
// overrides, and of members whose results are scala-library's views, which scalac reads as its
// own: the parents, the overrides' signatures, the synthesized tags and the view types.
class Offset(val offset: Int) extends PartialFunction[Int, Int]:
  def isDefinedAt(x: Int): Boolean = x > 0
  def apply(x: Int): Int = x + offset
  override def applyOrElse[A1 <: Int, B1 >: Int](x: A1, default: A1 => B1): B1 =
    if x > 0 then x + offset else default(x)

class Tag extends ClassTag[String]:
  def runtimeClass: Class[?] = classOf[String]
  override def unapply(x: Any): Option[String] = Some(x.toString)
  override def newArray(n: Int): Array[String] = Array.fill(n)("")

object Views:
  def seq: SeqView[Int] = List(1, 2).view
  def indexed: IndexedSeqView[Int] = Vector(1, 2).view
  def mapped(v: SeqView[Int]): SeqView[Int] = v.map(_ + 1)
  def filtered(v: SeqView[Int]): View[Int] = v.filter(_ > 1)
  def tagged[T](n: Int)(using ClassTag[T]): Array[T] = new Array[T](n)
  def strings: Array[String] = tagged[String](1)
  def lifted(p: PartialFunction[Int, Int]): Int => Option[Int] = p.lift
