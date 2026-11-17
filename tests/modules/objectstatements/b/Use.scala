package osb

import osa.*

// An upstream module's object named as a statement is evaluated for its effect, over the upstream's products as
// over its sources: its own statement, an inherited body, a trait's; and through the package whatever the class
// runs (an omitted default's effect too, which a bare name's statement would not make).
object Main:
  def main(args: Array[String]): Unit =
    println("own statement, by name"); OwnN
    println("inherited body, by name"); InheritedN
    println("trait body, by name"); ViaTraitN
    println("own statement, through the package"); osa.OwnP
    println("inherited body, through the package"); osa.InheritedP
    println("trait body, through the package"); osa.ViaTraitP
    println("omitted default, through the package"); osa.DefaultP
    println("quiet, through the package"); osa.QuietP
    println("end")
