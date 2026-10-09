package nb

import na.*

object Use:
  def name(c: Color): String = c match
    case Color.Red => "red"
    case Color.Green => "green"
    case Color.Blue => "blue"
  def main(args: Array[String]): Unit =
    println(Color.values.map(name).mkString(","))
    println(Color.valueOf("Green").ordinal)
    println(Planet.Earth.mass)
    val o: Opt[Int] = Opt.Some(1)
    println(o match { case Opt.Some(v) => v; case Opt.None => 0 })
