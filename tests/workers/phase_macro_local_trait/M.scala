import scala.quoted.*
import scala.compiletime.summonInline

trait TC:
  def get: Int

object M:
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
  inline def result: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(ofTrait(42))
  inline def resultOfClass: Int = ${ implOfClass }
  def implOfClass(using Quotes): Expr[Int] = Expr(ofClass(7))
