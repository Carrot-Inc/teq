package talias
val e = { println("  eager alias given"); 1 }
given Ordering[Int] = Ordering.Int.reverse
