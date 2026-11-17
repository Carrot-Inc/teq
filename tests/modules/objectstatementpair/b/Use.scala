package opb

import opa.*

// A statement naming an upstream module's object is judged by the class as this build reads it: over the
// upstream's products by its pickle, where the reference removes both statements below; built with the upstream's
// sources both objects are evaluated, as the reference's own build of the sources together evaluates them.
object Main:
  def main(args: Array[String]): Unit =
    println("an object with a parent's argument, by name"); Argument
    println("an object named through its owner object"); Outer.Inner
    println("end")
