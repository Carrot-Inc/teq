package etb

import eta.Relay

// A second hop: the downstream's export of the upstream's forwarders.
object Hop:
  export Relay.{renamed, add}

@main def run(): Unit =
  println(Relay.renamed())
  println(Relay.renamed(1))
  println(Relay.add(2))
  println(Hop.renamed())
  println(Hop.add(3))
  println(eta.Impl.original(4))
