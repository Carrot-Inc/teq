package web

object Findings:
  inline def id(x: Int): Int = x
  def viaF(x: Int): Int = id(x)
  def viaG(x: Int): Int = id(x)
  inline def inner(y: Int): Int = y
  inline def outer(y: Int): Int = inner(y)
  def caller(y: Int): Int = outer(y)

  class Base:
    def f(x: Int): Int = x
    def g(x: Int): Int = x
  class Sub extends Base:
    def f(x: String): Int = 1
    override def g(x: Int): Int = x + 1

  class Box(val v: Int)
  extension (c: Box)
    def a: Int = c.v
    def b: Int = c.v

  def first: Int = 1
  def shifted(x: Int): Int = x
  val namedCall = shifted(x = 2)

  def aliased: Int =
    type T = Int
    val t: T = 1
    t

  def locals: Int =
    class Local:
      def hello: Int = 1
    new Local().hello
  def helloZ: Int = 3

  def useTwice: Int = shapes.Geometry.twice(1)
