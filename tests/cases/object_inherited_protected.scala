// The protected members an object inherits are in scope in its body, bare, as in a class's:
// its statements, its methods, a closure and a nested class, and an eta-expansion (Scala.js's
// test bridge registers `handleMessage _` of its RPC base class).
abstract class Core:
  protected def handle(msg: String): Unit = println("handled " + msg)
  final protected def tag(n: Int): String = "#" + n
  protected val limit: Int = 3
  def register(f: String => Unit): Unit = f("x")

object Impl extends Core:
  handle("direct")
  println(tag(limit))
  register(handle _)
  register(handle)
  def later(): Unit = register(m => handle(m + "!"))
  class Inner:
    def show: String = tag(limit + 1)
  val lambda: Int => String = n => tag(n * limit)

@main def run(): Unit =
  Impl.later()
  println(new Impl.Inner().show)
  println(Impl.lambda(2))
