// The Seq-like reading of NodeList, HTMLCollection, FileList and TouchList that scalajs-dom gives through an
// implicit conversion. In package scala so that every file sees it; the receiver type ranks it above the
// UndefOr extensions of the same names.
package scala

import org.scalajs.dom.DOMList

extension [T](list: DOMList[T])
  @js("Array.from($0)")
  def toArray: Array[T]
  def toList: List[T] = list.toArray.toList
  def toSeq: Seq[T] = list.toArray.toSeq
  def toVector: Vector[T] = list.toArray.toVector
  def iterator: Iterator[T] = list.toArray.iterator
  def size: Int = list.length
  def isEmpty: Boolean = list.length == 0
  def nonEmpty: Boolean = list.length != 0
  def headOption: Option[T] = if list.length == 0 then None else Some(list(0))
  def lastOption: Option[T] = if list.length == 0 then None else Some(list(list.length - 1))
  def foreach[U](f: T => U): Unit = list.toArray.foreach(f)
  def map[B](f: T => B): Array[B] = list.toArray.map(f)
  def flatMap[B](f: T => IterableOnce[B]): Array[B] = list.toArray.flatMap(f)
  def filter(p: T => Boolean): Array[T] = list.toArray.filter(p)
  def find(p: T => Boolean): Option[T] = list.toArray.find(p)
  def exists(p: T => Boolean): Boolean = list.toArray.exists(p)
  def forall(p: T => Boolean): Boolean = list.toArray.forall(p)
  def indexWhere(p: T => Boolean): Int = list.toArray.indexWhere(p)
  def zipWithIndex: Array[(T, Int)] = list.toArray.zipWithIndex
  def mkString(sep: String): String = list.toArray.mkString(sep)
