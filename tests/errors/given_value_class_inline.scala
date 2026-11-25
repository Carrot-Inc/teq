// A value class in an inline given's body: the inline given is a class, not a static owner (E075).
// expect: 4:32: error: Value classes may not be a member of another class
trait T { def get: Int }
inline given G: T with { class V(val x: Int) extends AnyVal { def twice: Int = x * 2 }; def get: Int = new V(21).twice }
object Main { def main(args: Array[String]): Unit = println(G.get) }
