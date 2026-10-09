// Two warnings on one line, the same on both sides.
def f(b: Boolean): Int = { val a = b match { case false => 1 }; b match { case false => 1 } }
