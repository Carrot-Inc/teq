package rra

object A:
  def original(x: Int): Int = x + 1
  val base: Int = 40

// Exports under other names, which a downstream's export relays: its forwarders select them on
// `B` by the names `B` has them under.
object B:
  export A.{original as renamed, base as start}
