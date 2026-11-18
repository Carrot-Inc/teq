// A line left of its block that stands at a width an earlier line of the enclosing block stood at ends the
// block and is a statement of the enclosing one: here the width of an extension's methods, which stand under
// their header at a width of their own. scalac prints the two lines; without the extension it reports the line
// as matching no previous indentation width.
object O:
  extension (x: Int)
      def twice: Int = x * 2

  def f(a: Int): Unit =
          println(a)
      println("in O's body")

@main def main(): Unit = O.f(1)
