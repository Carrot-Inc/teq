// Item 9 of the JVM ABI alignment: a case object is its module class
// `X$`, which is its `Mirror.Singleton` (`fromProduct` answering it, and the bridge), with no companion of a case
// class's and no `copy`; a top-level one has the mirror class `X` of static forwarders.
// abi: classes apply unapply copy fromProduct toString hashCode canEqual productPrefix productArity productElement productElementName scala/deriving/Mirror$Singleton
package case_objects
case object X
sealed trait Signal
case object Stop extends Signal
object Lights:
  case object Amber extends Signal
