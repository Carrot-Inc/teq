package shop

class Counter:
  var n = 0
  def next(): Int = { n += 1; n }

package object words extends Greeting {
  val counter = new Counter
  def twice(s: String): String = greet(s) + " " + greet(s)
}

object After:
  def run(): String = words.twice("all")
