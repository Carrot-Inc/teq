object P { def f(implicit x: Int): Int = x; val y = f(/* hi */ 1); val z = f /* ( */ { 1 }; val w = f /* x */ (1); val v = f { /* ( */ 1 } }
