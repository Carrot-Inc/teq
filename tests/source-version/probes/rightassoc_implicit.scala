object P { infix def +:(implicit x: Int): Int = x; val y = (1) +: P; val z = /* ( */ 1 +: P }
