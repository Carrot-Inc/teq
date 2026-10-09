package eib

object Own:
  def half(x: Int): Int = x / 2

object P extends eia.Base:
  export eib.Own.*

object Use:
  import P.{*, given}
  def run: Int = twice(half(10)) + summon[Luck].n

@main def run(): Unit = println(Use.run)
