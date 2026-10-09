package jvb

import jva.Probe

def safely(s: String): String =
  try Probe.describe(s)
  catch case _: UnsupportedOperationException => "no file system"

@main def run(): Unit =
  println(safely("/"))
  println(safely("/no/such/file"))
