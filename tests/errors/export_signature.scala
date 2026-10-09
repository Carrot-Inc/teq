// expect: 9:8: error: object creation impossible, since def greet(n: Int): String in trait Greeter is not defined
// expect: 1 error found
trait Greeter:
  def greet(n: Int): String
  def name: String
object Impl:
  def greet: String = "hello"
  def name = "impl"
object G extends Greeter:
  export Impl.{greet, name}
@main def main(): Unit = println(G.name)
