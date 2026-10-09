//> using platform js
// Doubles hash as on Scala.js: a whole one as its Int, otherwise by its bits; `##` folds a whole
// one beyond the Int range as a Long.
case class D(d: Double)
@main def main(): Unit =
  println(1.0.hashCode + " " + 1.5.hashCode + " " + (-0.0).hashCode + " " + Double.NaN.hashCode + " " + Double.MaxValue.hashCode + " " + 1e10.hashCode)
  println(2.5.## + " " + 1e10.## + " " + (-0.0).## + " " + 3.0.##)
  println((1L, 2.5).hashCode + " " + (1.5, 2).hashCode + " " + D(2.5).hashCode + " " + D(1e10).hashCode + " " + D(-0.0).hashCode + " " + D(Double.NaN).hashCode)
  println(List(1.5, 2.5).hashCode + " " + List(1e10).hashCode + " " + Some(2.5).hashCode + " " + Set(1.5).hashCode + " " + Map(1 -> 2.5).hashCode)
