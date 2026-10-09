package eaa

// Expansions of a transparent method of the upstream made in the upstream's own bodies, whose
// anonymous classes a downstream reaches.
object Hello:
  def hello: String = Make.greeter("a").greet
  def twice: String = Make.greeter("b").greet + ", " + Make.greeter("c").greet
