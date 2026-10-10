package gosb

import gosa.{given, *}

@main def run(): Unit =
  println(top.self eq top)
  println(top.extra)
  println(Holder.held.self eq Holder.held)
  println(Holder.held.extra)
