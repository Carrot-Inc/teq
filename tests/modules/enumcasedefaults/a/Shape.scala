package ecd

// The companion of an enum's class case holds its synthesized `apply`, which takes the constructor's
// parameters and no getters of its own (`Desugar.classDef`): a call leaving a default out applies the
// constructor's getter on the companion (`Applications.findDefaultGetter` names a synthesized `apply`'s
// getter `<init>$default$N`), whichever compiler wrote the enum.
enum Shape {
  case Circle(r: Int = 2)
  case Rect(w: Int, h: Int = 3)
}
