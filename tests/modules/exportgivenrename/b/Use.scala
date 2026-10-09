package grb

import gra.{C, Show}
import C.given

@main def run(): Unit = println(summon[Show[Int]].show(7))
