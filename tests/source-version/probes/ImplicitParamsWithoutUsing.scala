object P { def f(implicit x: Int): Int = x; val x = f(1) }
