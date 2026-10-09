package nnest
val e = { println("  eager nested"); 1 }
object Outer:
  println("  outer")
  object Inner:
    println("  inner")
    val x = 5
