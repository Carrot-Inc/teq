package miu

import mia.M

@main def run(): Unit =
  println(M.viaArg)
  println(M.viaDefault)
  println(M.viaForwarder)
