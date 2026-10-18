object A { val a = 1; val b = 2 }
object P { import A._; import A.{a => x, b as y}; val c = x + y }
object Q { export A.{a => z}; export A._ }
