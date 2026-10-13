package cva

// A case class without a written companion has scalac's synthesized one (`Desugar.classDef`), a value
// of the products like a written object: `cva.F`, and per instance of a class `o.K`.
case class F(n: Int)
class O:
  case class K(n: Int)
