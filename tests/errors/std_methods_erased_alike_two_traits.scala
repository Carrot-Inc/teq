// expect: std_methods_erased_alike_two_traits.scala:11:7: error: class Both inherits apply as (s: String): String and as (n: Int): Int
// A class that takes `apply` of `java.util.function.Function` from one trait and `apply` of a function type from
// another: the two erase to the same parameter and result, and scalac 3.8.4 reports E120, a name clash after
// erasure, at the class.
trait Counted extends java.util.function.Function[Int, Int] {
  def apply(n: Int): Int = n + 10
}
trait Worded extends (String => String) {
  def apply(s: String): String = "str:" + s
}
class Both extends Counted with Worded
