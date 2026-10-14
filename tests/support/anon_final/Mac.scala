package anonfinal

import scala.quoted.*

trait PlainId[A]:
  def name: String
  def wrap(s: String): A
  def value(a: A): String

trait QuotedId[A]:
  def name: String
  def wrap(s: String): A
  def value(a: A): String

// The members of an anonymous class carry the flags the source gives them: `final` where written,
// in the class itself and in the copy a quote makes at an expansion's site (dotty's `javaFlags`,
// `sym.is(Final)`); a bridge stays without.
object Mac:
  inline def quoted: QuotedId[Int] = ${ quotedImpl }
  def quotedImpl(using Quotes): Expr[QuotedId[Int]] =
    '{
      new QuotedId[Int]:
        final def name: String = "quoted"
        final def wrap(s: String): Int = s.length
        def value(a: Int): String = a.toString
    }
  val plain: PlainId[Int] = new PlainId[Int]:
    final def name: String = "plain"
    def wrap(s: String): Int = s.length
    final def value(a: Int): String = a.toString
