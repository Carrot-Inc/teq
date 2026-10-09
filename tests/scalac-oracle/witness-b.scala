// witness-a.scala with another witness at the same place: its pinned difference no longer holds.
def f(u: Int | Double): Int = u match { case _: Int => 1 }
