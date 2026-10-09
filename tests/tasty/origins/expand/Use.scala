package ex

object Use:
  // Supplementary characters before the call: 𝔘𝔰𝔢
  def hello: String = Make.greeter("use").greet
  def twice: String = Make.greeter("one").greet + Make.greeter("two").greet
