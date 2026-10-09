package pub

import pua.*

object UsePolyUnused:
  val p: Parent { def run[A](a: Int): Int } = new Parent { def run[A](a: Int): Int = a + 1 }
  val kept: Parent { def run[A](a: Int): Int } = PolyUnused.keep(p)
  def main(args: Array[String]): Unit = println(kept.run[String](7))
