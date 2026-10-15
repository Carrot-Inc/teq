import scala.util.Random

// scala-library's `Random.alphanumeric` and `nextString`, as tyrian's random commands call them: the
// same characters for the same seed as scala-library's, drawn as the stream is read.
@main def run(): Unit =
  val r = new Random(42)
  println(r.alphanumeric.take(12).mkString)
  println(r.nextString(5).map(_.toInt).mkString(","))
  println(r.alphanumeric.take(3).toList)
  println(new Random(7).nextString(0).isEmpty)
