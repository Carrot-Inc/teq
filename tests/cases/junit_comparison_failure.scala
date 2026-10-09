//> using platform js
//> using dep org.scala-js:scalajs-junit-test-runtime_2.13:1.20.1
// interp-expected: js
// JUnit's `ComparisonFailure` as Scala.js's test runtime has it, which munit's comparison
// failures extend: the differing parts in brackets, the shared context cut to 20 characters;
// and `AssumptionViolatedException`, which munit's `assume` throws.
import org.junit.{AssumptionViolatedException, ComparisonFailure}

@main def run =
  println(new ComparisonFailure("names", "alpha beta", "alpha gamma").getMessage)
  println(new ComparisonFailure(null, "a" * 30 + "x" + "b" * 30, "a" * 30 + "y" + "b" * 30).getMessage)
  println(new ComparisonFailure("", "same", "same").getMessage)
  println(new ComparisonFailure("m", null, "x").getMessage)
  println(new ComparisonFailure("m", "ab", "abc").getMessage)
  val f = new ComparisonFailure("m", "e", "a")
  println(s"${f.getExpected} ${f.getActual} ${f.isInstanceOf[AssertionError]}")
  try throw new AssumptionViolatedException("not today")
  catch case e: RuntimeException => println(e.getMessage)
