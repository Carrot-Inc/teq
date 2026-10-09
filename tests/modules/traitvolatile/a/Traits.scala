package tva

// `@volatile` on a trait's vars, which the class mixing the trait in holds: its fields carry `ACC_VOLATILE` over
// the products as in the whole build (the flag read from the trait's pickle), a private var's under its expanded
// name (`tva$Flags$$hits`), a plain var's none.
trait Flags:
  @volatile var stop: Boolean = false
  @volatile private var hits: Int = 0
  var plain: Int = 1
  def hit(): Int = { hits += 1; hits }
