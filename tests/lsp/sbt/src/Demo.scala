object Demo:
  def greet(name: String): String = s"hello $name"
  val said: String = greet("you")
  val wrong: Int = "not an int"
