object Hello:
  val emoji = "😀"; val after = emoji
  def greet(name: String): String = "hello " + name
  val wrong: Int = "not an int"
  def main(args: Array[String]): Unit = println(greet("you") + after)
