import scala.quoted.*
object B:
  inline def m: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(2)
  val usesA: Int = A.m

@main def run(): Unit = println(A.usesB + B.usesA)
