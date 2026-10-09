// expect: Implicit classes must accept exactly one primary constructor parameter
// expect: A case class may not be defined as implicit
// expect: implicit modifier cannot be used for types or traits
// expect: expected '=', found '('

class A; class B
object E:
  def f(implicit a: A)(b: B): Int = 1
  implicit class Two(x: Int, y: Int) { def twice = x * 2 }
  implicit case class Cc(x: Int) { def thrice = x * 3 }
  implicit class NoParam { def q = 1 }
  implicit trait T
