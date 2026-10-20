// expect: exports are only allowed from objects and classes, they can not belong to local blocks
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
