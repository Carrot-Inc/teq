// A variable the constraint bounded only from above beyond its declared `Null` is instantiated
// from above (`Inferencing.instDirection`), so `y` is a `Box[String]`, no `Box[Null]`.
class Box[A]
def mk[A >: Null](f: A => Unit): Box[A] = new Box[A]
def run() =
  val y = mk((s: String) => ())
  val z: Box[Null] = y
// expect: 7:22: error: type mismatch: found Box[String], required Box[Null]
