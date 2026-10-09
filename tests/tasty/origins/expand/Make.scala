package ex

trait Greeter:
  def greet: String

object Make:
  // 𝔊 before the class: its offsets differ in bytes and UTF-16 units.
  transparent inline def greeter(inline name: String): Greeter = new Greeter:
    def greet: String = "𝔊 " + name
