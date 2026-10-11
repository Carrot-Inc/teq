package recm

// A refinement's own alias that a sibling names stays a member reference, so the refinement is a
// recursive type (`RecType.apply`, `closeOver`) in the pickle as in the source. The downstream's
// macro refuses any other shape; scalac's leg expands it over teq's pickles, teq's over its own
// and over scalac's (`over-scalac.txt`).
object Api:
  type S = AnyRef { type T = String; def f(): T }
  type R = AnyRef { type T; type U = T }
  type P = AnyRef { type T = String; def g(): String }
