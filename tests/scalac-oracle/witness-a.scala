// A witness scalac writes `_: String`, teq `String`: the difference tests/scalac-oracle.sh pins.
def f(u: Int | String): Int = u match { case _: Int => 1 }
