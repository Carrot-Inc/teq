// scala-library's `NameTransformer`: operator characters and other non-identifier characters
// in names, encoded and decoded.
import scala.reflect.NameTransformer

@main def run(): Unit =
  println(NameTransformer.encode("++"))
  println(NameTransformer.encode("a-b"))
  println(NameTransformer.encode("x€"))
  println(NameTransformer.decode("$plus$plus"))
  println(NameTransformer.decode("$u20ACsign"))
  println(NameTransformer.decode("$less$init$greater"))
  println(NameTransformer.decode("C<init>"))
  println(NameTransformer.decode("a$b"))
  println(NameTransformer.encode("áb").toList.map(_.toInt).mkString(","))
  println(NameTransformer.encode("a b"))
  println(NameTransformer.decode("$uabcd"))
  println(NameTransformer.decode("$u0abc").toList.map(_.toInt).mkString(","))
  println(NameTransformer.decode("$uABCD").toList.map(_.toInt).mkString(","))
  println(NameTransformer.decode("$uAZZZ"))
