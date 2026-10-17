object P { infix def f(implicit x: Int): Int = x; val y = P f (1); val z = P f 1 }
