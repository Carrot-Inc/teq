import scala.quoted.*
import scala.compiletime.summonInline
trait TC:
  def get: Int
object A:
  inline def read: Int = summonInline[TC].get
  def value(n: Int): Int =
    given TC with
      def get = n
    trait Local:
      def get: Int = read
    class Child extends Local
    Child().get
  val trigger = B.result
  inline def result: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(value(42))
