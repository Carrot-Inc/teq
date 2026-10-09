package usb

import usa.*

class C extends Spelled(1)(using 6) with Hidden("h")(using "!")

@main def run(): Unit =
  val c = C()
  println(s"${c.answer} ${c.shown} ${c.`using$1`}")
