// A type variable an application leaves open is the one its result holds nonvariantly
// (`Inferencing.interpolateTypeVars`): a covariant one is instantiated from below at once, widened
// (`widenInferred`), so a singleton expected of a selection on it is refused; an inferred `val`
// forces what stays open (`Namer.inferredResultType`'s `fullyDefinedType`); an ascription widens.
class CBox[+T](val zero: T)
class Box[T](val zero: T)
def cbox[T](t: T): CBox[T] = new CBox(t)
def box[T](t: T): Box[T] = new Box(t)
def run() =
  val a: 1 = cbox(1).zero
  val b: 1 = List(1).head
  val stored = box(1)
  val c: 1 = stored.zero
  val d: 1 = box((1: Int)).zero
  val e: 1 = box(1).zero
// expect: 10:14: error: type mismatch: found Int, required 1
// expect: 11:14: error: type mismatch: found Int, required 1
// expect: 13:14: error: type mismatch: found Int, required 1
// expect: 14:14: error: type mismatch: found Int, required 1
// absent: 15:
