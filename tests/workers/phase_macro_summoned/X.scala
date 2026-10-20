import scala.quoted.*
trait TC:
  def get: Int
trait Box:
  def v: Int
object X:
  inline def read: Int = compiletime.summonInline[TC].get
  def value(n: Int): Int =
    given TC with
      def get = n
    trait Local:
      def get: Int = read
    class Child extends Local
    Child().get
  inline given box: Box = ${impl}
  def impl(using Quotes): Expr[Box] =
    val n = value(42)
    '{ new Box { def v = ${Expr(n)} } }
