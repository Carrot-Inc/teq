class InputContext[T](value: T):
  def pair[R](tag: Int, f: (T, T) => R): R = f(value, value)
  def pair[R](tag: Boolean, f: (T, T) => R): R = f(value, value)

import scala.annotation.targetName
object CommonInputLists:
  type Number = Int
  def pair[A](tag: Int, f: (Int, String) => A): A = f(2, "ab")
  def pair[A](tag: Boolean, f: (Number, String) => A): A = f(3, "cde")
  def first[A, B](a: A, b: B): A = a
  def triple[A](tag: Int, f: (Int, String, Boolean) => A): A = f(4, "xy", true)
  def triple[A](tag: Boolean, f: (Int, String, Boolean) => A): A = f(5, "xyz", false)
  def differing(tag: Int, f: (Int, String) => String): String = f(6, "pq")
  def differing(tag: Boolean, f: (String, Int) => String): String = f("rs", 7)
  def generic[A](tag: Int, value: A, f: (A, Int) => String): String = f(value, 8)
  def generic[A](tag: Boolean, value: A, f: (A, Int) => String): String = f(value, 9)
  def two(x: Int, y: String): Int = x + y.length
  def arity(f: (Int, String) => Int): Int = f(10, "s")
  @targetName("arityThree") def arity(f: (Int, String, Boolean) => Int): Int = f(11, "s", true)

  def wide[A](tag: Int, f: (Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, String) => A): A = f(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, "xyz")
  def wide[A](tag: Boolean, f: (Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, String) => A): A = f(2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, "ab")

  def outer[T](value: T): String =
    val context = new InputContext[T](value)
    context.pair(true, (x, y) => x.toString + y.toString)

@main def runCommonInputLists(): Unit =
  println(CommonInputLists.pair(0, CommonInputLists.first))
  println(CommonInputLists.pair(true, (x, y) => x + y.length))
  println(CommonInputLists.triple(0, (x, y, z) => if z then x + y.length else 0))
  println(CommonInputLists.triple(true, (x, y, z) => if z then 0 else x + y.length))
  println(CommonInputLists.differing(0, (x: Int, y: String) => y + x))
  println(CommonInputLists.differing(true, (x: String, y: Int) => x + y))
  println(CommonInputLists.generic(0, "a", (x: String, y: Int) => x + y))
  println(CommonInputLists.generic(true, "b", (x: String, y: Int) => x + y))
  println(CommonInputLists.arity((x: Int, y: String) => x + y.length))
  println(CommonInputLists.arity((x: Int, y: String, z: Boolean) => x + y.length))
  println(CommonInputLists.wide(0, (a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22, s) => a1 + a22 + s.length))
  println(CommonInputLists.wide(true, (a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22, s) => a1 + a22 + s.length))
  println(CommonInputLists.outer("z"))
  println(CommonInputLists.arity(CommonInputLists.two))
