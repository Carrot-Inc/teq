// expect: 12:43: error: type mismatch: found (Box#value : Box#T), required ?1.T
// The design pass's Paths probe: each `fresh()` is a path of its own, which the qualifier skolems of
// `TypeAssigner.maybeSkolemizePrefix` and `TypeOps.asSeenFrom` keep apart (TypeOps.scala 35-56); scalac refuses it
// (E007, `where: ?1 is an unknown value of type Box`), teq approximates paths with inference variables and accepts.
// Moves with type-representation's landing 4.
trait Box:
  type T
  val value: T
  def accept(x: T): Unit
object Paths:
  def fresh(): Box = ???
  def bad(): Unit = fresh().accept(fresh().value)
