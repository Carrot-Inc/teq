// Members that Java defines with parentheses take both forms, as under scalac; a def declared
// with () is otherwise called with them, and eta-expands where a function is expected.
package javaparens

class Counter:
  private var n = 0
  def next(): Int =
    n += 1
    n
  def twice(f: () => Int): Int = f() + f()
  override def toString: String = s"Counter($n)"
  override def hashCode: Int = n

@main def main(): Unit =
  val c = Counter()
  println(c.next())
  println(c.twice(c.next))
  println(c.twice(() => c.next()))
  println(c.toString())
  println(c.toString)
  println(c.hashCode())
  println(List(1).toString())
  println(List(1).hashCode() == List(1).hashCode)
  println("abc".length())
  println("abc".length)
  println("abc".isEmpty())
  println(" a ".strip())
  println(" ".isBlank())
  println("abc".hashCode() == "abc".hashCode)
  val sb = StringBuilder("xy")
  println(sb.length())
  println(sb.length)
  println(sb.toString())
  println(sb.result())
  println(Math.random() < 1.0)
  println(Math.random < 1.0)
  val it = List(1, 2).iterator
  println(it.next())
  println(it.hasNext)
