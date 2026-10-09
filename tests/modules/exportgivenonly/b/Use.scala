package egb

import ega.{B, Show}
import B.given

@main def run(): Unit = println(summon[Show[Int]].show(42))
