// Operations on the native js types whose members cannot be declared on the type itself. They sit in package
// scala, which every file sees, next to the UndefOr ones; the compiler picks the extension whose receiver fits
// best, so a js.Dictionary gets these and not the A | Unit ones of the same name.
package scala

import scala.scalajs.js

/** The marker the app names in `js.Object & Dynamic`; teq's only Dynamic is the JS one. */
type Dynamic = js.Dynamic

extension [A](dict: js.Dictionary[A])
  def get(key: String): Option[A] = if dict.contains(key) then Some(dict(key)) else None
  def getOrElseUpdate(key: String, default: => A): A =
    if dict.contains(key) then dict(key)
    else
      val value = default
      dict(key) = value
      value
  @js("($1 in $0)")
  def contains(key: String): Boolean
  @js("(delete $0[$1])")
  def delete(key: String): Boolean
  @js("Object.keys($0)")
  def keys: Array[String]
  @js("Object.values($0)")
  def values: Array[A]
  @js("Object.keys($0).length")
  def size: Int
  def isEmpty: Boolean = dict.size == 0
  def nonEmpty: Boolean = dict.size != 0
  def toList: List[(String, A)] = dict.keys.toList.map(key => (key, dict(key)))
  def toMap = dict.toList.toMap
  def foreach[U](f: ((String, A)) => U): Unit = dict.toList.foreach(f)
  def -=(key: String): js.Dictionary[A] =
    dict.delete(key)
    dict

extension [A, B](tuple: js.Tuple2[A, B])
  def toScalaTuple: (A, B) = (tuple._1, tuple._2)
