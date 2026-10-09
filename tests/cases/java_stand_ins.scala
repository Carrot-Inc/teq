// A definition in package scala takes the place of the standard library's stand-in for a
// java.lang member, as it shadows java.lang.Integer or java.lang.Comparable under scalac.
package scala

object Integer:
  val MAX_VALUE: Int = 2147483647
  def parseInt(s: String): Int = s.trim.toInt * 10

trait Comparable[T]:
  def compareTo(that: T): Int
  def isBefore(that: T): Boolean = compareTo(that) < 0

final class Version(val n: Int) extends Comparable[Version]:
  def compareTo(that: Version): Int = n - that.n

@main def javaStandIns(): Unit =
  println(Integer.parseInt(" 4 "))
  println(Integer.MAX_VALUE)
  println(Version(1).isBefore(Version(2)))
  println(Version(3).compareTo(Version(1)))
  println(String.valueOf(12) + "!")
