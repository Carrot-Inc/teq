class V(val text: String) extends AnyVal

object Main:
  def reason(): V = new V("missing")

  def fail(): Unit =
    throw new IllegalArgumentException(reason()
      .text)

  def main(args: Array[String]): Unit =
    try fail()
    catch case e: IllegalArgumentException => println(e.getMessage)
