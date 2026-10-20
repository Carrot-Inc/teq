object D:
  final case class P(x: Int)

  given Show[P] = Show.derived

  inline def line: Int = sourcecode.Line.generate.value
