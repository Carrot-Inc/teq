package jlib

// A Scala class whose construction prints, the superclass of a Java class (Ledger.java); and the object a Java
// interface's initializer calls (Meter.java).
abstract class Tally:
  println("tally-init")

object Signal:
  def note(): Int =
    println("meter-init")
    7
