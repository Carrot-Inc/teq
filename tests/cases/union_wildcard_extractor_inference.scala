type Scalar = String | Int | Boolean | Double
type Input = Scalar | Option[Scalar]
def visit(input: Input): String = input match {
  case selected: Option[?] => selected match {
    case Some(value) =>
      val scalar: Scalar = value
      if scalar == null then "null" else "some"
    case _ => "empty"
  }
  case _ => "scalar"
}
@main def unionWildcardExtractorInference(): Unit = {
  println(visit(Some("text")))
  println(visit(Some(1)))
  println(visit(Some(true)))
  println(visit(Some(2.0)))
  println(visit(None))
  println(visit(0))
}
