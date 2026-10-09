package cyc

// Finding `Base` searches this import, whose object in turn exports `Utils`.
import cyc.Prelude.*

trait Base:
  def base: Int = 1

object Utils extends Base:
  def util: Int = helper + base

@main def run(): Unit =
  println(util)
  println(Prelude.util + Prelude.base)
