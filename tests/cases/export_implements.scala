trait Greeter:
  def greet: String
  def name: String
  def version: Int
  def shout(s: String, times: Int): String
  def all: String = greet + "/" + name + "/" + version + "/" + shout("x", 2)
object Impl:
  def greet = "hello from Impl"
  val name = "impl"
  val version = 3
  def shout(s: String, times: Int): String = (s * times).toUpperCase
def topGreet: String = "top"
val topVersion: Int = 7
object G extends Greeter:
  export Impl.{greet, name, version, shout}
object H extends Greeter:
  export Impl.{name, shout}
  def greet = topGreet
  val version = topVersion
def use(g: Greeter) = g.all
@main def main(): Unit =
  println(use(G))
  println(use(H))
  println(G.greet + " " + G.version + " " + H.name)
