package sumord

// A sealed trait's written companion is its sum mirror (`SymUtils.useCompanionAsSumMirror`), whose
// `ordinal` `SyntheticMembers.addMirrorSupport` enters whether or not this module summons the mirror:
// a downstream module's summon calls it (a scalac consumer of the products too).
sealed trait S
object S
case class A(x: Int) extends S
case object B extends S
