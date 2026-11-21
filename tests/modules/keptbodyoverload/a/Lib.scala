package kbo

class A(val v: Int)
class B(val v: Int)

// Overloads a body the library keeps calls: top-level and in an object, told apart by the first list and by the
// using list. Over the library's products the call is the alternative the pickle names.
def take(x: Int): String = "i" + x
def take(s: String): String = "s" + s
def pick(x: Int)(using a: A): String = "a" + x + a.v
def pick(x: Int)(using b: B): String = "b" + x + b.v
object O:
  def take(x: Int)(using a: A): String = "oa" + x + a.v
  def take(s: String)(using a: A): String = "os" + s + a.v
  def pick(x: Int)(using a: A): String = "pa" + x + a.v
  def pick(x: Int)(using b: B): String = "pb" + x + b.v

object Lib:
  given A = A(100)
  def top: String = take(1)
  def topUsing: String = pick(1)(using B(2))
  def obj: String = O.take(1)(using A(1))
  def objUsing: String = O.pick(1)(using B(2))
