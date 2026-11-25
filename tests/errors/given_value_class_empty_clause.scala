// A value class in a given with an empty clause (`given G: () => T`), a class: not a static owner (E075).
// expect: 4:26: error: Value classes may not be a member of another class
trait T { def get: Int }
given G: () => T { class V(val x: Int) extends AnyVal { def twice: Int = x * 2 }; def get: Int = new V(21).twice }
object Main { def main(args: Array[String]): Unit = println(G.get) }
