package exa

object Engine:
  def start(power: Int): String = "vroom " + power
  def stop: Unit = ()
  val cylinders: Int = 4
  type Fuel = String

package parts:
  def bolt(size: Int): Int = size * 2
  val nut: String = "nut"

object Gauge:
  def level: Int = 1

// A wildcard export from an object, the members of a package, and a renamed export nothing
// uses.
class Car:
  export Engine.*
  export parts.{bolt, nut}
  export Gauge.{level => count}
