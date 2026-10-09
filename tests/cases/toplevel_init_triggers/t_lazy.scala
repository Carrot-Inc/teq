package tlazy
val e = { println("  eager lazy"); 1 }
lazy val lz: Int = { println("  lazy"); 3 }
