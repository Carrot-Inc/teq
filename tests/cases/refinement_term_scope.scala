// A path inside a refinement names the refinement's own vals and its parent's (the refinement
// class's members, `RecThis.a`), and a val member of a refinement conforms as the value's member
// seen through it (`TypeComparer.hasMatchingMember`: `tp1.member(name).info`).
trait P { val a: String; val b: Any }
type R = P { val b: a.type }
type S = AnyRef { val a: String; val b: a.type }
class K extends P { val a: String = "ok"; val b: a.type = a }
def read(r: R): r.a.type = r.b

@main def run(): Unit =
  val k = K()
  val r: R = k
  val s: String = read(r)
  val direct: P { val b: k.a.type } = k
  println(s + " " + direct.b)
