package glib

import glia.*

@main def run(): Unit =
  println(Counter.both())
  println(Counter.plain())
  println(Counter.local())
  println(topLevel())
