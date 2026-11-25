// Value classes in owners that are not static: a class, a trait, givens with a parameter, a type parameter, an inline given, a method and a local object (E075).
// expect: 10:20: error: Value classes may not be a member of another class
// expect: 11:18: error: Value classes may not be a member of another class
// expect: 12:47: error: Value classes may not be a member of another class
// expect: 13:39: error: Value classes may not be a member of another class
// expect: 14:43: error: Value classes may not be a member of another class
// expect: 15:61: error: Value classes may not be a local class
// expect: 15:108: error: Value classes may not be a member of another class
trait T
class Host { class V(val x: Int) extends AnyVal }
trait Tr { class V(val x: Int) extends AnyVal }
object P { given G(using Int): T with { class V(val x: Int) extends AnyVal } }
object Q { given G[A]: T with { class V(val x: Int) extends AnyVal } }
object I { inline given G: T with { class V(val x: Int) extends AnyVal } }
object Main { def main(args: Array[String]): Unit = { class V(val x: Int) extends AnyVal; object O { class W(val x: Int) extends AnyVal }; println(21) } }
