//> using platform jvm
// `ofPattern`'s numbers made as `appendValue` makes them, whatever the letter: a width of 19 taken
// and printed, one of 20 refused when the pattern is read, with the builder's message; the second
// review's `FormatterVariation`.
import java.time.LocalDateTime
import java.time.format.DateTimeFormatter

object Main:
  def main(args: Array[String]): Unit =
    val at = LocalDateTime.of(2024, 1, 2, 3, 4, 5, 6)
    for letter <- List("n", "y", "u"); count <- List(3, 19, 20, 21) do
      val shown =
        try DateTimeFormatter.ofPattern(letter * count).format(at)
        catch case e: IllegalArgumentException => e.getClass.getName + ": " + e.getMessage
      println(s"$letter$count=$shown")
    println(try { DateTimeFormatter.ofPattern("n" * 20); "made" } catch case e: IllegalArgumentException => e.getMessage)
