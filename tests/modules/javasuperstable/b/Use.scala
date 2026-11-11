package jsb

import jsa.*

inline def ignore(x: Any): Unit = ()

@main def run(): Unit =
  ignore(new Thr)
  println(new Thread().getName)
  ignore(new Jl)
  new Jl
  println("|")
