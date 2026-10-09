package st

// Overrides whose types are the inherited members' own.
class Impl extends Show:
  def show(x: A | B): String = "impl"
  def both(x: A & B, y: A | B): Unit = ()
