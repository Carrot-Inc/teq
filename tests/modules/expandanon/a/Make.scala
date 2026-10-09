package eaa

trait Greeter:
  def greet: String

object Make:
  transparent inline def greeter(inline name: String): Greeter = new Greeter:
    def greet: String = "hi " + name
