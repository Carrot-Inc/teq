object P { var a = 0; val u = (a = 3); infix def f(a: Int, b: Int): Int = a; val v = P f (a = 1, b = 2) }
