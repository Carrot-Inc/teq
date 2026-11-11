import scala.compiletime.ops

trait Domain:
  type Elem
  def value: Elem

object Numbers extends Domain:
  type Elem = Int
  def value = 4

object Main:
  type N = Int
  val n: N = 4
  type S = n.type
  def get: S = n
  def value(d: Domain): d.Elem = d.value
  def first: ops.any.ToString[7] = "7"
  def second: ops.any.ToString[8] = "8"
  def add(n: 4): Int = n + 1
  def wider(n: 4): Long = n
  def main(args: Array[String]): Unit =
    println(value(Numbers) + 1)
    println(first + second)
    println(wider(4) + add(4))
    println(get + 1)
