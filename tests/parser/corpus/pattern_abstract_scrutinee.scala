// A typed pattern on a scrutinee of an abstract type binds the pattern's type intersected with
// the scrutinee's (`String & U`), as scalac types the binder `body.tpe & pt` where the one does
// not conform to the other: the binder is still a `U` past the case.
package patternabstract

trait Holder:
  type T
  def value: T
  def describe: String = value match
    case s: String => "string " + s.length + " " + same(s)
    case n: Int => "int " + (n + 1) + " " + same(n)
    case _ => "other"
  def same(t: T): Boolean = t == value

class StringHolder(val value: String) extends Holder:
  type T = String

class IntHolder(val value: Int) extends Holder:
  type T = Int

object Main:
  def keep[U](b: U): U = b match
    case s: String => s
    case sb: StringBuilder => sb
    case _ => b
  def sized[U](b: U): Int = b match
    case s: String => s.length
    case xs: List[?] => xs.size
    case _ => -1
  def pair[U](b: U): (U, String) = b match
    case s: String => (s, s.toUpperCase)
    case other => (other, other.toString)

  def main(args: Array[String]): Unit =
    println(s"${keep("abc")} ${keep(42)} ${keep(new StringBuilder("xy")).length}")
    println(s"${sized("abcd")} ${sized(List(1, 2, 3))} ${sized(3.5)}")
    println(s"${pair("ab")} ${pair(7)}")
    println(new StringHolder("hey").describe + " | " + new IntHolder(4).describe)
