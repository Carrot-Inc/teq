// A class that derives from `scala.Dynamic` and calls of its dynamic methods written as members: in a method's
// body and in inline methods' bodies here; a second module reads them from this one's products, expands the
// inline ones and writes its own.
package lib

import scala.language.dynamics

class Record(val tag: String) extends Dynamic:
  def selectDynamic(name: String): String = s"$tag.select($name)"
  def applyDynamic(name: String)(args: Any*): String = s"$tag.apply($name)(${args.mkString(",")})"
  def applyDynamicNamed(name: String)(args: (String, Any)*): String =
    s"$tag.named($name)(${args.map((k, v) => s"$k=$v").mkString(",")})"
  def updateDynamic(name: String)(value: Any): Unit = println(s"$tag.update($name)($value)")

object Reads:
  def field(r: Record): String = r.field
  inline def call(r: Record, n: Int): String = r.call(n, "x")
  inline def named(r: Record): String = r.named(a = 1, b = "two")
  inline def assign(r: Record, v: Int): Unit = r.slot = v
  transparent inline def chained(r: Record): String = r.first + "|" + r.second(2)
