import scala.quoted.*
import scala.compiletime.summonInline

trait TC:
  def get: Int

object Producer:
  inline def read: Int = summonInline[TC].get
  def ofTrait(n: Int): Int =
    given TC with
      def get = n
    trait Local:
      def get: Int = read
    class Child extends Local
    Child().get
  def ofClass(n: Int): Int =
    given TC with
      def get = n
    class Local:
      def get: Int = read
    class Child extends Local
    Child().get
  transparent inline def result: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(ofTrait(42))
  transparent inline def resultOfClass: Int = ${ implOfClass }
  def implOfClass(using Quotes): Expr[Int] = Expr(ofClass(7))
