package b

import lib.Keys

val pad: List[Int] = List(1, 2, 3).map(_ + 1).filter(_ > 2).map(_ * 2)
def second(xs: List[Int]): String = Keys.keyed(xs, _.toString)
