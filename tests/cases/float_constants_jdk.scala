// java.lang.Float's constants, the JDK's, which scala-library's scala.Float reads (a linked build's
// Float.PositiveInfinity was undefined without them).
@main def m =
  import java.lang.{Float as F}
  println(Seq(F.MAX_VALUE, F.MIN_VALUE, F.MIN_NORMAL, F.POSITIVE_INFINITY, F.NEGATIVE_INFINITY, F.NaN).map(F.floatToIntBits(_)).mkString(","))
