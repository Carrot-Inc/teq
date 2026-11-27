package a2
import scala.quoted.*
class C(val x: Int = 31) { def n: Int = x }
trait T extends C
object Api {
  inline def mk: T = ${ mkImpl }
  def mkImpl(using Quotes): Expr[T] = '{ new T {} }
}
