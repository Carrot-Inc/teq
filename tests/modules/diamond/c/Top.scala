package dic

import dia.*
import dib.*

object Top:
  def main(args: Array[String]): Unit =
    println(Zoo.all.map(a => a.name + " " + a.sound).mkString(", "))
    println(Dog("fido").copy(name = "odi"))
