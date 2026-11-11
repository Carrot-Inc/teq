package a

import lib.Keys

def first(xs: List[Int]): String = Keys.keyed(xs, _.toString)
