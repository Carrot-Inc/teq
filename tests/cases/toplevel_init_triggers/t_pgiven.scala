package tpg
val e = { println("  eager param given"); 1 }
given listOrd(using o: Ordering[Int]): Ordering[List[Int]] = Ordering.by((l: List[Int]) => l.headOption.getOrElse(0))
