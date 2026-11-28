package dinc

// A getter takes the clauses before its parameter's as its own parameters, their flags masked as
// `Desugar.toMethParam` masks them (`GivenOrImplicit | Erased | Tracked`): `f$default$2`'s copy of `a`
// is neither `inline` nor defaulted, in the pickle a downstream build reads, by teq or by scalac.
class Inline:
  inline def f(inline a: Int = 1)(inline b: Int = a): Int = a + b
