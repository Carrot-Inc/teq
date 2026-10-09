// expect: exports are only supported in the body of an object, trait or class
// expect: export clauses are not supported in a given

trait Show[A]:
  def show(a: A): String

object Source:
  def one: Int = 1

given Show[Int]:
  export Source.*
  def show(a: Int): String = a.toString

def f(): Int =
  export Source.one
  one
