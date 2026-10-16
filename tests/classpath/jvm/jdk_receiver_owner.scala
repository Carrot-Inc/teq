// A JDK member is named on the qualifier's class as a program's is (dotty's `BCodeBodyBuilder.genApply`):
// a member `StringBuilder` inherits from its package-private `AbstractStringBuilder`, an `ArrayList`'s
// members its interfaces' defaults declare, `Object`'s member on a JDK class, a map's override, an
// exception's inherited `getMessage` (the std's `@jvm` template of `Throwable`'s). tests/classfile.scala
// compares each reference with scalac's.
package jdkowner

object Main:
  def sbLength(sb: java.lang.StringBuilder): Int = sb.length()
  def sbChar(sb: java.lang.StringBuilder): Char = sb.charAt(0)
  def sbAppend(sb: java.lang.StringBuilder): java.lang.StringBuilder = sb.append("x")
  def listSize(a: java.util.ArrayList[Int]): Int = a.size()
  def listEmpty(a: java.util.ArrayList[Int]): Boolean = a.isEmpty()
  def listStream(a: java.util.ArrayList[Int]): Long = a.stream().count()
  def listString(a: java.util.ArrayList[Int]): String = a.toString
  def mapDefault(m: java.util.HashMap[String, Int]): Int = m.getOrDefault("a", 1)
  def linkedSize(m: java.util.LinkedHashMap[String, Int]): Int = m.size()
  def dequeEmpty(d: java.util.ArrayDeque[Int]): Boolean = d.isEmpty
  def message(t: IllegalStateException): String = t.getMessage
  def main(args: Array[String]): Unit =
    val sb = new java.lang.StringBuilder("ab")
    val a = new java.util.ArrayList[Int]()
    a.add(1)
    println(List(sbLength(sb), sbChar(sb), sbAppend(sb), listSize(a), listEmpty(a), listStream(a), listString(a)))
    println(List(mapDefault(new java.util.HashMap[String, Int]()), linkedSize(new java.util.LinkedHashMap[String, Int]()), dequeEmpty(new java.util.ArrayDeque[Int]()), message(new IllegalStateException("m"))))
