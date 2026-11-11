package spb

import spa.*

inline def ignore(x: Any): Unit = ()

@main def run(): Unit =
  ignore(new Q); println("|")
  ignore(new QT); println("|")
  ignore(new Plain(2)); println("|")
